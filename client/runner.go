package client

import (
	"context"
	"fmt"
	"math"
	"net/url"
	"strings"
	"sync"
	"time"

	"github.com/karol-broda/funnel/shared"
)

const maxReconnectDelay = 30 * time.Second

func Run(tunnelID, serverURL, localAddr, token string, shutdown <-chan struct{}) {
	logger := shared.GetTunnelLogger("client.runner", tunnelID)
	logger.Info().Str("local_addr", localAddr).Str("server_url", serverURL).Bool("has_token", token != "").Msg("starting tunnel client with reconnection logic")

	reconnectAttempts := 0

	for {
		select {
		case <-shutdown:
			logger.Info().Msg("shutdown signal received, stopping client runner.")
			return
		default:
			// continue
		}

		c := New(tunnelID, serverURL, localAddr, token)

		logger.Info().Int("attempt", reconnectAttempts+1).Msg("attempting to connect to server")
		err := c.connect(c.ctx)

		if err != nil {
			errorCategory := categorizeConnectionError(err)
			logger.Error().Err(err).Str("error_category", errorCategory).Msg("connection failed")

			select {
			case <-shutdown:
				continue
			case <-time.After(getReconnectDelay(reconnectAttempts)):
				reconnectAttempts++
				continue
			}
		}

		logger.Info().Msg("connected successfully")
		reconnectAttempts = 0

		if u, err := url.Parse(c.ServerURL); err != nil {
			logger.Error().Err(err).Msg("failed to parse server url for public url display")
		} else {
			publicURL := fmt.Sprintf("http://%s.%s", c.TunnelID, u.Host)
			logger.Info().Str("public_url", publicURL).Msg("tunnel is available")
		}

		var wg sync.WaitGroup
		wg.Add(2)

		runCtx, cancelRun := context.WithCancel(context.Background())

		go func() {
			defer wg.Done()
			c.readPump()
			cancelRun()
		}()

		go func() {
			defer wg.Done()
			c.writePump()
			cancelRun()
		}()

		select {
		case <-shutdown:
			logger.Info().Msg("shutdown during active connection, closing.")
			c.Close()
		case <-runCtx.Done():
			logger.Warn().Msg("connection lost, will attempt to reconnect.")
		}

		wg.Wait()
		c.Close()
	}
}

func getReconnectDelay(attempts int) time.Duration {
	delay := time.Duration(math.Pow(2, float64(attempts))) * time.Second
	if delay > maxReconnectDelay {
		return maxReconnectDelay
	}
	return delay
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
