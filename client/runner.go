package client

import (
	"time"
	"tunneling/shared"
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
			if reconnectAttempts >= maxReconnectAttempts {
				logger.Error().Err(err).Int("max_attempts", maxReconnectAttempts).Msg("maximum reconnection attempts reached, backing off")
				time.Sleep(30 * time.Second)
				reconnectAttempts = 0
				continue
			}

			retryDelay := time.Duration(reconnectAttempts) * baseRetryDelay
			if retryDelay > 30*time.Second {
				retryDelay = 30 * time.Second
			}

			logger.Error().Err(err).Int("attempt", reconnectAttempts).Dur("retry_in", retryDelay).Msg("connection failed, retrying")
			time.Sleep(retryDelay)
			continue
		}

		reconnectAttempts = 0
		logger.Info().Msg("connected successfully")
		logger.Info().Str("public_url", "http://"+c.TunnelID+".<server-domain>").Msg("tunnel is available")

		connectionStart := time.Now()
		c.handleRequests()
		connectionDuration := time.Since(connectionStart)

		logger.Warn().Dur("connection_duration", connectionDuration).Msg("connection lost, reconnecting")
		time.Sleep(2 * time.Second)
	}
}
