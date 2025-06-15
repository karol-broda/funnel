package client

import (
	"bytes"
	"context"
	"errors"
	"io"
	"net/http"
	"strings"
	"time"

	"github.com/gorilla/websocket"
	"github.com/karol-broda/funnel/shared"
)

func (c *Client) readPump() {
	defer func() {
		c.requestWg.Wait()
		c.Close()
	}()
	logger := shared.GetTunnelLogger("client.handler", c.TunnelID)

	logger.Info().Msg("starting request handler loop")

	transport := &http.Transport{
		MaxIdleConns:        10,
		MaxIdleConnsPerHost: 5,
		IdleConnTimeout:     30 * time.Second,
		MaxConnsPerHost:     10,
	}

	httpClient := &http.Client{
		Timeout:   30 * time.Second,
		Transport: transport,
		CheckRedirect: func(req *http.Request, via []*http.Request) error {
			return http.ErrUseLastResponse
		},
	}

	logger.Debug().
		Dur("http_timeout", httpClient.Timeout).
		Int("max_conns_per_host", 10).
		Int("max_idle_conns", 10).
		Msg("http client configured")

	c.setupHeartbeat()

	requestCount := 0
	for {
		select {
		case <-c.ctx.Done():
			logger.Info().Msg("read pump shutting down due to context cancellation")
			return
		default:
			var msg shared.Message
			readStart := time.Now()
			err := c.Conn.ReadJSON(&msg)
			readDuration := time.Since(readStart)

			if err != nil {
				if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseNormalClosure) {
					logger.Error().Err(err).Dur("read_duration", readDuration).Msg("unexpected websocket read error")
				} else {
					logger.Info().Dur("read_duration", readDuration).Msg("websocket connection closed gracefully")
				}
				return
			}

			logger.Debug().Dur("message_read_time", readDuration).Str("message_type", msg.Type).Msg("received message from server")

			switch msg.Type {
			case "request":
				requestCount++
				logger.Info().
					Int("request_count", requestCount).
					Str("request_id", msg.RequestID).
					Str("method", msg.Method).
					Str("path", msg.Path).
					Int("body_size", len(msg.Body)).
					Msg("received request from server")

				c.requestWg.Add(1)
				go func(m shared.Message) {
					defer c.requestWg.Done()
					c.processRequest(httpClient, m)
				}(msg)
			case "request_cancel":
				logger.Info().Str("request_id", msg.RequestID).Msg("received request cancellation")
				c.ongoingRequestsMu.Lock()
				if cancel, ok := c.ongoingRequests[msg.RequestID]; ok {
					cancel()
					delete(c.ongoingRequests, msg.RequestID)
				}
				c.ongoingRequestsMu.Unlock()
			default:
				logger.Debug().Str("message_type", msg.Type).Msg("ignoring unhandled message type")
			}
		}
	}
}

func (c *Client) setupHeartbeat() {
	logger := shared.GetTunnelLogger("client.handler", c.TunnelID)

	c.Conn.SetPongHandler(func(string) error {
		c.updateLastPong()
		logger.Debug().Msg("pong received from server")
		return nil
	})

	go func() {
		ticker := time.NewTicker(30 * time.Second)
		defer ticker.Stop()

		for {
			select {
			case <-c.ctx.Done():
				logger.Debug().Msg("context cancelled, stopping ping routine")
				return
			case <-ticker.C:
				if c.Conn == nil {
					logger.Debug().Msg("connection is nil, stopping ping routine")
					return
				}

				lastPong := c.getLastPong()
				if time.Since(lastPong) > 90*time.Second {
					logger.Warn().
						Dur("time_since_last_pong", time.Since(lastPong)).
						Msg("no pong received recently, connection may be stale")
					c.Close()
					return
				}

				select {
				case c.outgoingMessages <- &shared.Message{Type: "ping"}:
					logger.Debug().Msg("ping message queued")
				case <-c.ctx.Done():
					logger.Debug().Msg("context cancelled while queuing ping, stopping ping routine")
					return
				default:
					logger.Warn().Msg("outgoing message channel full, cannot queue ping")
				}
			}
		}
	}()

	logger.Debug().Msg("heartbeat mechanism setup completed")
}

func (c *Client) writePump() {
	defer func() {
		c.Close()
	}()
	logger := shared.GetTunnelLogger("client.handler", c.TunnelID)
	logger.Info().Msg("starting writer loop")

	for msg := range c.outgoingMessages {
		if msg.Type == "ping" {
			if err := c.Conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				logger.Error().Err(err).Msg("failed to send ping")
				return
			}
			logger.Debug().Msg("ping sent to server")
			continue
		}

		c.connMu.Lock()
		err := c.Conn.WriteJSON(msg)
		c.connMu.Unlock()

		if err != nil {
			logger.Error().Err(err).Msg("failed to write json message")
			return
		}

		if msg.RequestID != "" {
			logger.Debug().
				Str("request_id", msg.RequestID).
				Str("type", msg.Type).
				Msg("message sent to server")
		}
	}
	logger.Info().Msg("writer loop finished")
}

func (c *Client) processRequest(httpClient *http.Client, msg shared.Message) {
	logger := shared.GetRequestLogger("client.handler", c.TunnelID, msg.RequestID)

	select {
	case c.requestSemaphore <- struct{}{}:
		defer func() { <-c.requestSemaphore }()
	case <-c.ctx.Done():
		logger.Warn().Msg("shutting down, not processing new request")
		return
	}

	reqCtx, cancel := context.WithTimeout(c.ctx, 30*time.Second)
	defer cancel()

	c.ongoingRequestsMu.Lock()
	c.ongoingRequests[msg.RequestID] = cancel
	c.ongoingRequestsMu.Unlock()

	defer func() {
		c.ongoingRequestsMu.Lock()
		delete(c.ongoingRequests, msg.RequestID)
		c.ongoingRequestsMu.Unlock()
	}()

	processStart := time.Now()
	logger.Debug().
		Str("method", msg.Method).
		Str("path", msg.Path).
		Int("header_count", len(msg.Headers)).
		Int("body_size", len(msg.Body)).
		Int("concurrent_requests", len(c.requestSemaphore)).
		Msg("processing request")

	req, err := http.NewRequestWithContext(reqCtx, msg.Method, "http://"+c.LocalAddr+msg.Path, bytes.NewReader(msg.Body))
	if err != nil {
		logger.Error().Err(err).Msg("failed to create request")
		if reqCtx.Err() == nil {
			c.sendError(msg.RequestID, http.StatusInternalServerError, "failed to create request")
		}
		return
	}

	c.setRequestHeaders(req, msg.Headers)

	logger.Debug().
		Int("concurrent_requests", len(c.requestSemaphore)).
		Int("header_count", len(req.Header)).
		Str("local_url", req.URL.String()).
		Msg("forwarding request to local service")

	upstreamResponseTimeStart := time.Now()
	resp, err := httpClient.Do(req)

	if err != nil {
		if errors.Is(err, context.Canceled) || errors.Is(err, context.DeadlineExceeded) {
			logger.Warn().Err(err).Msg("request to local service was canceled or timed out")
			return
		}
		logger.Error().Err(err).Msg("failed to make request to local service")
		c.sendError(msg.RequestID, http.StatusBadGateway, "local service connection failed")
		return
	}
	defer resp.Body.Close()

	logger.Debug().
		Int("status_code", resp.StatusCode).
		Dur("upstream_response_time", time.Since(upstreamResponseTimeStart)).
		Int64("content_length", resp.ContentLength).
		Msg("received response from local service")

	bodyReadStart := time.Now()
	body, err := io.ReadAll(resp.Body)
	bodyReadDuration := time.Since(bodyReadStart)

	if err != nil {
		logger.Error().Err(err).
			Dur("body_read_duration", bodyReadDuration).
			Msg("failed to read response body")
		c.sendError(msg.RequestID, http.StatusBadGateway, "failed to read response body")
		return
	}

	totalProcessTime := time.Since(processStart)
	logger.Info().
		Int("status_code", resp.StatusCode).
		Int("response_size", len(body)).
		Dur("total_process_time", totalProcessTime).
		Dur("upstream_time", time.Since(upstreamResponseTimeStart)).
		Dur("body_read_time", bodyReadDuration).
		Msg("request processed successfully")

	select {
	case <-c.ctx.Done():
		logger.Info().Str("request_id", msg.RequestID).Msg("shutting down, not sending response")
		return
	default:
		c.sendResponse(msg.RequestID, resp.StatusCode, resp.Header, body)
	}
}

func (c *Client) setRequestHeaders(req *http.Request, headers map[string][]string) {
	req.Header = make(http.Header)

	for k, values := range headers {
		if c.shouldSkipHeader(k) {
			continue
		}
		if strings.ToLower(k) == "host" {
			continue
		}
		for _, value := range values {
			req.Header.Add(k, value)
		}
	}
	req.Host = c.LocalAddr
}

func (c *Client) shouldSkipHeader(headerName string) bool {
	lower := strings.ToLower(headerName)
	return lower == "connection" ||
		lower == "keep-alive" ||
		lower == "proxy-authenticate" ||
		lower == "proxy-authorization" ||
		lower == "te" ||
		lower == "trailer" ||
		lower == "transfer-encoding" ||
		lower == "upgrade" ||
		lower == "proxy-connection"
}

func (c *Client) sendResponse(requestID string, status int, headers http.Header, body []byte) {
	logger := shared.GetRequestLogger("client.handler", c.TunnelID, requestID)

	respHeaders := make(map[string][]string)
	for k, v := range headers {
		if !c.shouldSkipHeader(k) {
			respHeaders[k] = v
		}
	}

	respMsg := &shared.Message{
		Type:      "response",
		RequestID: requestID,
		Status:    status,
		Headers:   respHeaders,
		Body:      body,
	}

	select {
	case c.outgoingMessages <- respMsg:
		sendDuration := time.Since(time.Now())
		logger.Debug().
			Int("status_code", status).
			Int("header_count", len(respHeaders)).
			Int("response_size", len(body)).
			Dur("send_duration", sendDuration).
			Msg("response queued successfully")
	default:
		logger.Warn().Msg("outgoing message channel full, dropping response")
	}
}

func (c *Client) sendError(requestID string, statusCode int, error string) {
	logger := shared.GetRequestLogger("client.handler", c.TunnelID, requestID)
	logger.Error().Int("status_code", statusCode).Str("error", error).Msg("sending error response")
	errMessage := &shared.Message{
		Type:      "response",
		RequestID: requestID,
		Status:    statusCode,
		Headers:   http.Header{"Content-Type": []string{"text/plain"}},
		Body:      []byte(error),
	}
	select {
	case c.outgoingMessages <- errMessage:
	default:
		logger.Warn().Int("status", statusCode).Msg("outgoing message channel full, dropping error response")
	}
}
