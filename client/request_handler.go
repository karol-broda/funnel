package client

import (
	"bytes"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"

	"tunneling/shared"
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

		if msg.Type != "request" {
			logger.Debug().Str("message_type", msg.Type).Msg("ignoring non-request message")
			continue
		}

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
	}
}

func (c *Client) processRequest(httpClient *http.Client, msg shared.Message) {
	logger := shared.GetRequestLogger("client.handler", c.TunnelID, msg.RequestID)

	processStart := time.Now()
	logger.Debug().
		Str("method", msg.Method).
		Str("path", msg.Path).
		Int("header_count", len(msg.Headers)).
		Int("body_size", len(msg.Body)).
		Msg("processing request")

	localURL := fmt.Sprintf("http://%s%s", c.LocalAddr, msg.Path)
	req, err := http.NewRequest(msg.Method, localURL, bytes.NewReader(msg.Body))
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

	filteredHeaders := 0
	totalHeaders := 0

	for k, v := range headers {
		totalHeaders += len(v)
		if strings.HasPrefix(k, "X-") || k == "Host" || k == "Connection" {
			filteredHeaders += len(v)
			logger.Debug().Str("header", k).Msg("filtering out header")
			continue
		}
		for _, val := range v {
			req.Header.Add(k, val)
		}
	}

	logger.Debug().
		Int("total_headers", totalHeaders).
		Int("filtered_headers", filteredHeaders).
		Int("forwarded_headers", totalHeaders-filteredHeaders).
		Msg("processed request headers")
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

	response := shared.Message{
		Type:      "response",
		RequestID: requestID,
		Status:    status,
		Headers: map[string][]string{
			"Content-Type": {"text/plain"},
		},
		Body: []byte(http.StatusText(status)),
	}

	if err := c.Conn.WriteJSON(&response); err != nil {
		logger.Error().Err(err).Int("error_status", status).Msg("failed to send error response")
	} else {
		logger.Debug().Int("error_status", status).Msg("error response sent successfully")
	}
}
