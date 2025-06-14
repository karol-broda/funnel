package client

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"

	"github.com/karol-broda/funnel/shared"
)

func (c *Client) handleRequests() {
	logger := shared.GetTunnelLogger("client.handler", c.TunnelID)

	logger.Info().Msg("starting request handler loop")

	httpClient := &http.Client{
		Timeout: 30 * time.Second,
		CheckRedirect: func(req *http.Request, via []*http.Request) error {
			return http.ErrUseLastResponse
		},
	}

	logger.Debug().Dur("http_timeout", httpClient.Timeout).Msg("http client configured")

	requestCount := 0
	for {
		var msg shared.Message
		readStart := time.Now()
		err := c.Conn.ReadJSON(&msg)
		readDuration := time.Since(readStart)

		if err != nil {
			logger.Error().Err(err).Dur("read_duration", readDuration).Msg("websocket read error")
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

			go func(m shared.Message) {
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

func (c *Client) processRequest(httpClient *http.Client, msg shared.Message) {
	logger := shared.GetRequestLogger("client.handler", c.TunnelID, msg.RequestID)

	ctx, cancel := context.WithCancel(context.Background())
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
		Msg("processing request")

	localURL := fmt.Sprintf("http://%s%s", c.LocalAddr, msg.Path)
	req, err := http.NewRequestWithContext(ctx, msg.Method, localURL, bytes.NewReader(msg.Body))
	if err != nil {
		logger.Error().Err(err).Str("local_url", localURL).Msg("failed to create request")
		c.sendError(msg.RequestID, 500)
		return
	}

	c.setRequestHeaders(req, msg.Headers)

	logger.Debug().Str("local_url", localURL).Msg("forwarding request to local service")

	requestStart := time.Now()
	resp, err := httpClient.Do(req)
	requestDuration := time.Since(requestStart)

	if err != nil {
		logger.Error().Err(err).
			Str("local_url", localURL).
			Dur("request_duration", requestDuration).
			Msg("request to local service failed")

		// check if the request was canceled by the client
		if ctx.Err() == context.Canceled {
			logger.Info().Msg("request to local service was canceled")
			// no need to send an error response, the original connection is gone
			return
		}

		c.sendError(msg.RequestID, 502)
		return
	}
	defer resp.Body.Close()

	logger.Debug().
		Int("status_code", resp.StatusCode).
		Dur("upstream_response_time", requestDuration).
		Int64("content_length", resp.ContentLength).
		Msg("received response from local service")

	bodyReadStart := time.Now()
	body, err := io.ReadAll(resp.Body)
	bodyReadDuration := time.Since(bodyReadStart)

	if err != nil {
		logger.Error().Err(err).
			Dur("body_read_duration", bodyReadDuration).
			Msg("failed to read response body")
		c.sendError(msg.RequestID, 500)
		return
	}

	totalProcessTime := time.Since(processStart)
	logger.Info().
		Int("status_code", resp.StatusCode).
		Int("response_size", len(body)).
		Dur("total_process_time", totalProcessTime).
		Dur("upstream_time", requestDuration).
		Dur("body_read_time", bodyReadDuration).
		Msg("request processed successfully")

	c.sendResponse(msg.RequestID, resp.StatusCode, resp.Header, body)
}

func (c *Client) setRequestHeaders(req *http.Request, headers map[string][]string) {
	logger := shared.GetTunnelLogger("client.handler", c.TunnelID)

	totalHeaders := 0
	forwardedHeaders := 0
	hostHeaderSet := false

	for k, v := range headers {
		totalHeaders += len(v)

		if k == "Host" {
			if !hostHeaderSet {
				req.Host = c.LocalAddr
				hostHeaderSet = true
				logger.Debug().
					Str("original_host", v[0]).
					Str("local_host", req.Host).
					Msg("host header updated for local service")
			}
			continue
		}

		if c.shouldSkipHeader(k) {
			logger.Debug().Str("header", k).Msg("skipping connection-specific header")
			continue
		}

		for _, val := range v {
			req.Header.Add(k, val)
			forwardedHeaders++
		}
	}

	if !hostHeaderSet {
		req.Host = c.LocalAddr
		logger.Debug().
			Str("local_host", req.Host).
			Msg("host header set to local service address")
	}

	logger.Debug().
		Int("total_headers", totalHeaders).
		Int("forwarded_headers", forwardedHeaders).
		Int("skipped_headers", totalHeaders-forwardedHeaders).
		Msg("processed request headers")
}

func (c *Client) shouldSkipHeader(headerName string) bool {
	// convert to lowercase for case-insensitive comparison
	lower := strings.ToLower(headerName)

	switch lower {
	case "connection":
		return true
	case "upgrade":
		return true
	case "proxy-connection":
		return true
	case "proxy-authenticate":
		return true
	case "proxy-authorization":
		return true
	case "te":
		return true
	case "trailer":
		return true
	case "transfer-encoding":
		return true
	}

	// forward all other headers, including:
	// - X-Forwarded-* headers (important for upstream services to know about original request)
	// - X-Real-IP (real client IP)
	// - Standard HTTP headers
	// - Custom application headers

	return false
}

func (c *Client) sendResponse(requestID string, status int, headers http.Header, body []byte) {
	logger := shared.GetRequestLogger("client.handler", c.TunnelID, requestID)

	sendStart := time.Now()
	response := shared.Message{
		Type:      "response",
		RequestID: requestID,
		Status:    status,
		Headers:   headers,
		Body:      body,
	}

	c.mu.Lock()
	defer c.mu.Unlock()
	if err := c.Conn.WriteJSON(&response); err != nil {
		sendDuration := time.Since(sendStart)
		logger.Error().Err(err).
			Dur("send_duration", sendDuration).
			Int("response_size", len(body)).
			Msg("failed to send response")
	} else {
		sendDuration := time.Since(sendStart)
		logger.Debug().
			Dur("send_duration", sendDuration).
			Int("status_code", status).
			Int("response_size", len(body)).
			Int("header_count", len(headers)).
			Msg("response sent successfully")
	}
}

func (c *Client) sendError(requestID string, status int) {
	logger := shared.GetRequestLogger("client.handler", c.TunnelID, requestID)

	logger.Warn().Int("error_status", status).Msg("sending error response")

	errorBody := []byte(http.StatusText(status))
	response := shared.Message{
		Type:      "response",
		TunnelID:  c.TunnelID,
		RequestID: requestID,
		Status:    status,
		Headers: map[string][]string{
			"Content-Type":   {"text/plain"},
			"Content-Length": {fmt.Sprintf("%d", len(errorBody))},
		},
		Body: errorBody,
	}
	c.mu.Lock()
	defer c.mu.Unlock()
	if err := c.Conn.WriteJSON(&response); err != nil {
		logger.Error().Err(err).Int("error_status", status).Msg("failed to send error response")
	}
}
