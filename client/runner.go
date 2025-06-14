package client

import (
	"fmt"
	"net/url"
	"strings"
	"time"

	"github.com/karol-broda/funnel/shared"
)

func (c *Client) runWithReconnection() {
	logger := shared.GetTunnelLogger("client.runner", c.TunnelID)

	logger.Info().Str("local_addr", c.LocalAddr).Str("server_url", c.ServerURL).Msg("starting tunnel client with reconnection logic")

	reconnectAttempts := 0
	maxReconnectAttempts := 10
	baseRetryDelay := 5 * time.Second

	for {
		reconnectAttempts++
		logger.Info().Int("attempt", reconnectAttempts).Str("server_url", c.ServerURL).Msg("attempting to connect to server")

		err := c.connect()
		if err != nil {
			// categorize the error for better diagnostics
			errorCategory := categorizeConnectionError(err)

			if reconnectAttempts >= maxReconnectAttempts {
				logger.Error().
					Err(err).
					Str("error_category", errorCategory).
					Int("max_attempts", maxReconnectAttempts).
					Msg("maximum reconnection attempts reached, backing off")
				time.Sleep(30 * time.Second)
				reconnectAttempts = 0
				continue
			}

			retryDelay := time.Duration(reconnectAttempts) * baseRetryDelay
			if retryDelay > 30*time.Second {
				retryDelay = 30 * time.Second
			}

			logger.Error().
				Err(err).
				Str("error_category", errorCategory).
				Int("attempt", reconnectAttempts).
				Dur("retry_in", retryDelay).
				Msg("connection failed, retrying")
			time.Sleep(retryDelay)
			continue
		}

		reconnectAttempts = 0
		logger.Info().Msg("connected successfully")

		u, err := url.Parse(c.ServerURL)
		if err != nil {
			logger.Error().Err(err).Msg("failed to parse server url for public url display")
		} else {
			publicURL := fmt.Sprintf("http://%s.%s", c.TunnelID, u.Host)
			logger.Info().Str("public_url", publicURL).Msg("tunnel is available")
		}

		connectionStart := time.Now()
		c.handleRequests()
		connectionDuration := time.Since(connectionStart)

		logger.Warn().
			Dur("connection_duration", connectionDuration).
			Msg("connection lost, reconnecting")

		// cleanup connection state
		if c.Conn != nil {
			c.Conn.Close()
			c.Conn = nil
		}

		time.Sleep(2 * time.Second)
	}
}

func categorizeConnectionError(err error) string {
	if err == nil {
		return "unknown"
	}

	errStr := strings.ToLower(err.Error())

	switch {
	case strings.Contains(errStr, "timeout"):
		return "timeout"
	case strings.Contains(errStr, "refused"):
		return "connection_refused"
	case strings.Contains(errStr, "no such host"):
		return "dns_resolution"
	case strings.Contains(errStr, "network"):
		return "network_error"
	case strings.Contains(errStr, "tls"):
		return "tls_error"
	case strings.Contains(errStr, "websocket"):
		return "websocket_error"
	default:
		return "other"
	}
}
