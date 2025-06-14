package server

import (
	"fmt"
	"io"
	"net/http"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	"github.com/karol-broda/funnel/shared"
)

type TunnelRouter struct {
	server *Server

	hostCache sync.Map // map[string]string (host -> subdomain)

	requests    int64
	cacheHits   int64
	cacheMisses int64

	bufferPool sync.Pool
}

func NewTunnelRouter(server *Server) *TunnelRouter {
	logger := shared.GetLogger("server.router")

	router := &TunnelRouter{
		server: server,
		bufferPool: sync.Pool{
			New: func() interface{} {
				b := make([]byte, 1024)
				return &b
			},
		},
	}

	logger.Info().
		Int("buffer_pool_size", 1024).
		Msg("tunnel router initialized")

	return router
}

func (tr *TunnelRouter) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	requestStart := time.Now()
	requestID := fmt.Sprintf("%d", time.Now().UnixNano())
	logger := shared.GetLogger("server.router")

	atomic.AddInt64(&tr.requests, 1)
	currentRequests := atomic.LoadInt64(&tr.requests)

	logger.Info().
		Str("request_id", requestID).
		Str("method", r.Method).
		Str("host", r.Host).
		Str("path", r.URL.Path).
		Str("remote_addr", r.RemoteAddr).
		Str("user_agent", r.UserAgent()).
		Int64("total_requests", currentRequests).
		Msg("incoming HTTP request")

	if len(r.Header["Upgrade"]) > 0 && r.Header["Upgrade"][0] == "websocket" {
		logger.Debug().Str("request_id", requestID).Msg("routing to websocket handler")
		tr.server.HandleWebSocket(w, r)
		return
	}

	subdomainStart := time.Now()
	subdomain := tr.getSubdomain(r.Host)
	subdomainDuration := time.Since(subdomainStart)

	if subdomain == "" {
		processingDuration := time.Since(requestStart)
		logger.Warn().
			Str("request_id", requestID).
			Str("host", r.Host).
			Dur("processing_time", processingDuration).
			Msg("tunnel not found - invalid host")
		http.Error(w, "tunnel not found", http.StatusNotFound)
		return
	}

	logger.Debug().
		Str("request_id", requestID).
		Str("subdomain", subdomain).
		Dur("subdomain_extraction_time", subdomainDuration).
		Msg("extracted subdomain from host")

	tunnel, exists := tr.server.GetTunnel(subdomain)
	if !exists {
		processingDuration := time.Since(requestStart)
		logger.Warn().
			Str("request_id", requestID).
			Str("subdomain", subdomain).
			Dur("processing_time", processingDuration).
			Msg("tunnel not found - no active tunnel")
		http.Error(w, "tunnel not found", http.StatusNotFound)
		return
	}

	logger.Debug().
		Str("request_id", requestID).
		Str("tunnel_id", subdomain).
		Msg("found active tunnel, routing request")

	tr.handleTunnelRequest(w, r, tunnel, requestID, requestStart)
}

func (tr *TunnelRouter) getSubdomain(host string) string {
	logger := shared.GetLogger("server.router")

	if cached, ok := tr.hostCache.Load(host); ok {
		atomic.AddInt64(&tr.cacheHits, 1)
		hits := atomic.LoadInt64(&tr.cacheHits)
		logger.Debug().
			Str("host", host).
			Str("cached_subdomain", cached.(string)).
			Int64("total_cache_hits", hits).
			Msg("subdomain cache hit")
		return cached.(string)
	}

	atomic.AddInt64(&tr.cacheMisses, 1)
	misses := atomic.LoadInt64(&tr.cacheMisses)

	logger.Debug().
		Str("host", host).
		Int64("total_cache_misses", misses).
		Msg("subdomain cache miss")

	subdomain := tr.extractSubdomain(host)

	if subdomain != "" {
		tr.hostCache.Store(host, subdomain)
		logger.Debug().
			Str("host", host).
			Str("extracted_subdomain", subdomain).
			Msg("subdomain cached")
	} else {
		logger.Debug().
			Str("host", host).
			Msg("no valid subdomain found")
	}

	return subdomain
}

func (tr *TunnelRouter) extractSubdomain(host string) string {
	if len(host) == 0 {
		return ""
	}

	hostLen := len(host)
	for i := hostLen - 1; i >= 0; i-- {
		if host[i] == ':' {
			host = host[:i]
			break
		}
		if host[i] < '0' || host[i] > '9' {
			break
		}
	}

	for i := 0; i < len(host); i++ {
		if host[i] == '.' {
			if i == 0 {
				return ""
			}
			return host[:i]
		}
	}

	return ""
}

func (tr *TunnelRouter) handleTunnelRequest(w http.ResponseWriter, r *http.Request, tunnel *Tunnel, requestID string, requestStart time.Time) {
	logger := shared.GetRequestLogger("server.router", tunnel.ID, requestID)

	bodyReadStart := time.Now()
	body, err := tr.readBody(r)
	bodyReadDuration := time.Since(bodyReadStart)

	if err != nil {
		processingDuration := time.Since(requestStart)
		logger.Error().Err(err).
			Dur("body_read_duration", bodyReadDuration).
			Dur("total_processing_time", processingDuration).
			Msg("failed to read request body")
		http.Error(w, "failed to read body", http.StatusInternalServerError)
		return
	}

	logger.Debug().
		Int("body_size", len(body)).
		Dur("body_read_time", bodyReadDuration).
		Msg("request body read successfully")

	headers := tr.prepareForwardingHeaders(r)

	msg := &shared.Message{
		Type:      "request",
		TunnelID:  tunnel.ID,
		RequestID: requestID,
		Method:    r.Method,
		Path:      r.URL.String(),
		Headers:   headers,
		Body:      body,
	}

	channelStart := time.Now()
	respChan := tunnel.registerResponseChannel(msg.RequestID)
	defer tunnel.unregisterResponseChannel(msg.RequestID)
	channelDuration := time.Since(channelStart)

	sendStart := time.Now()
	if err := tunnel.SendMessage(msg); err != nil {
		sendDuration := time.Since(sendStart)
		processingDuration := time.Since(requestStart)
		logger.Error().Err(err).
			Dur("send_duration", sendDuration).
			Dur("total_processing_time", processingDuration).
			Msg("failed to send message to tunnel")
		http.Error(w, "tunnel connection lost", http.StatusBadGateway)
		return
	}
	sendDuration := time.Since(sendStart)

	logger.Debug().
		Dur("channel_setup_time", channelDuration).
		Dur("message_send_time", sendDuration).
		Msg("request forwarded to tunnel")

	waitStart := time.Now()
	timeout := 30 * time.Second

	select {
	case resp := <-respChan:
		waitDuration := time.Since(waitStart)
		if resp != nil {
			logger.Info().
				Int("status_code", resp.Status).
				Int("response_size", len(resp.Body)).
				Dur("tunnel_response_time", waitDuration).
				Dur("total_request_time", time.Since(requestStart)).
				Msg("received response from tunnel")
			tr.writeResponse(w, resp, requestID, requestStart)
		} else {
			processingDuration := time.Since(requestStart)
			logger.Error().
				Dur("wait_duration", waitDuration).
				Dur("total_processing_time", processingDuration).
				Msg("received nil response from tunnel")
			http.Error(w, "tunnel connection lost", http.StatusBadGateway)
		}
	case <-time.After(timeout):
		processingDuration := time.Since(requestStart)
		logger.Error().
			Dur("timeout_duration", timeout).
			Dur("wait_duration", time.Since(waitStart)).
			Dur("total_processing_time", processingDuration).
			Msg("request timeout waiting for tunnel response")
		http.Error(w, "request timed out", http.StatusGatewayTimeout)

	case <-r.Context().Done():
		processingDuration := time.Since(requestStart)
		logger.Warn().
			Dur("wait_duration", time.Since(waitStart)).
			Dur("total_processing_time", processingDuration).
			Msg("client closed connection")
		http.Error(w, "client closed connection", 499)
	}
}

func (tr *TunnelRouter) prepareForwardingHeaders(r *http.Request) map[string][]string {
	logger := shared.GetLogger("server.router")

	headers := make(map[string][]string)
	for k, v := range r.Header {
		headers[k] = make([]string, len(v))
		copy(headers[k], v)
	}

	clientIP := tr.getClientIP(r)

	currentIP := r.RemoteAddr
	if colonIndex := strings.LastIndex(currentIP, ":"); colonIndex != -1 {
		currentIP = currentIP[:colonIndex]
	}
	if strings.HasPrefix(currentIP, "[") && strings.HasSuffix(currentIP, "]") {
		currentIP = currentIP[1 : len(currentIP)-1]
	}

	if existingForwardedFor := headers["X-Forwarded-For"]; len(existingForwardedFor) > 0 {
		headers["X-Forwarded-For"] = []string{existingForwardedFor[0] + ", " + currentIP}
	} else {
		headers["X-Forwarded-For"] = []string{currentIP}
	}

	if r.Host != "" {
		headers["X-Forwarded-Host"] = []string{r.Host}
	}

	proto := "http"
	if r.TLS != nil {
		proto = "https"
	}
	if existingProto := r.Header.Get("X-Forwarded-Proto"); existingProto != "" {
		proto = existingProto
	}
	headers["X-Forwarded-Proto"] = []string{proto}

	headers["X-Real-IP"] = []string{clientIP}

	if r.Header.Get("X-Forwarded-Server") == "" {
		headers["X-Forwarded-Server"] = []string{r.Host}
	}

	logger.Debug().
		Str("client_ip", clientIP).
		Str("forwarded_host", r.Host).
		Str("forwarded_proto", proto).
		Str("forwarded_for", headers["X-Forwarded-For"][0]).
		Msg("prepared forwarding headers")

	return headers
}

func (tr *TunnelRouter) getClientIP(r *http.Request) string {
	if xff := r.Header.Get("X-Forwarded-For"); xff != "" {
		if commaIndex := strings.Index(xff, ","); commaIndex != -1 {
			return strings.TrimSpace(xff[:commaIndex])
		}
		return strings.TrimSpace(xff)
	}

	if xri := r.Header.Get("X-Real-IP"); xri != "" {
		return strings.TrimSpace(xri)
	}

	if xf := r.Header.Get("X-Forwarded"); xf != "" {
		if forIndex := strings.Index(xf, "for="); forIndex != -1 {
			start := forIndex + 4
			end := strings.Index(xf[start:], ";")
			if end == -1 {
				end = len(xf)
			} else {
				end += start
			}
			return strings.TrimSpace(xf[start:end])
		}
	}

	ip := r.RemoteAddr

	if strings.HasPrefix(ip, "[") {
		endBracket := strings.Index(ip, "]")
		if endBracket != -1 {
			return ip[1:endBracket]
		}
	}

	if colonIndex := strings.LastIndex(ip, ":"); colonIndex != -1 {
		if strings.Count(ip, ":") > 1 {
			return ip
		}
		return ip[:colonIndex]
	}

	return ip
}

func (tr *TunnelRouter) readBody(r *http.Request) ([]byte, error) {
	logger := shared.GetLogger("server.router")

	if r == nil || r.Body == nil {
		logger.Debug().Msg("no request body to read")
		return nil, nil
	}

	defer r.Body.Close()

	if r.ContentLength == 0 {
		logger.Debug().Msg("zero content length, skipping body read")
		return nil, nil
	}

	if r.ContentLength > 0 && r.ContentLength < 1024 {
		logger.Debug().
			Int64("content_length", r.ContentLength).
			Msg("using buffer pool for small body")

		bufPtr := tr.bufferPool.Get().(*[]byte)
		buf := *bufPtr
		defer func() {
			for i := range buf {
				buf[i] = 0
			}
			tr.bufferPool.Put(bufPtr)
		}()

		n, err := io.ReadFull(r.Body, buf[:r.ContentLength])
		if err != nil {
			logger.Error().Err(err).Int("bytes_read", n).Msg("failed to read small body")
			return nil, err
		}
		return buf[:n], nil
	}

	logger.Debug().
		Int64("content_length", r.ContentLength).
		Msg("reading large body with ReadAll")

	body, err := io.ReadAll(r.Body)
	if err != nil {
		logger.Error().Err(err).
			Int64("content_length", r.ContentLength).
			Msg("error reading large body")
		return nil, err
	}

	logger.Debug().
		Int("bytes_read", len(body)).
		Msg("large body read successfully")
	return body, nil
}

func (tr *TunnelRouter) writeResponse(w http.ResponseWriter, resp *shared.Message, requestID string, requestStart time.Time) {
	logger := shared.GetLogger("server.router")

	writeStart := time.Now()

	headerCount := 0
	for k, values := range resp.Headers {
		headerCount += len(values)
		for _, v := range values {
			w.Header().Add(k, v)
		}
	}

	if resp.Status > 0 {
		w.WriteHeader(resp.Status)
	}

	bytesWritten := 0
	if len(resp.Body) > 0 {
		n, err := w.Write(resp.Body)
		bytesWritten = n
		if err != nil {
			logger.Error().Err(err).
				Str("request_id", requestID).
				Int("bytes_written", bytesWritten).
				Msg("error writing response body")
			return
		}
	}

	writeDuration := time.Since(writeStart)
	totalDuration := time.Since(requestStart)

	logger.Info().
		Str("request_id", requestID).
		Int("status_code", resp.Status).
		Int("header_count", headerCount).
		Int("bytes_written", bytesWritten).
		Dur("write_duration", writeDuration).
		Dur("total_request_duration", totalDuration).
		Msg("response written successfully")
}

func (tr *TunnelRouter) InvalidateCache(tunnelID string) {
	logger := shared.GetLogger("server.router")

	invalidatedCount := 0
	tr.hostCache.Range(func(key, value interface{}) bool {
		if value.(string) == tunnelID {
			tr.hostCache.Delete(key)
			invalidatedCount++
		}
		return true
	})

	logger.Info().
		Str("tunnel_id", tunnelID).
		Int("invalidated_entries", invalidatedCount).
		Msg("cache invalidated for tunnel")
}

func (tr *TunnelRouter) GetStats() map[string]interface{} {
	requests := atomic.LoadInt64(&tr.requests)
	hits := atomic.LoadInt64(&tr.cacheHits)
	misses := atomic.LoadInt64(&tr.cacheMisses)

	hitRate := float64(0)
	if hits+misses > 0 {
		hitRate = float64(hits) / float64(hits+misses) * 100
	}

	stats := map[string]interface{}{
		"total_requests": requests,
		"cache_hits":     hits,
		"cache_misses":   misses,
		"cache_hit_rate": fmt.Sprintf("%.1f%%", hitRate),
	}

	logger := shared.GetLogger("server.router")
	logger.Debug().
		Int64("total_requests", requests).
		Int64("cache_hits", hits).
		Int64("cache_misses", misses).
		Float64("hit_rate", hitRate).
		Msg("router statistics requested")

	return stats
}
