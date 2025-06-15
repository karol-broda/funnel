package client

import (
	"context"
	"net/url"
	"time"

	"github.com/karol-broda/funnel/shared"

	"github.com/gorilla/websocket"
)

func (c *Client) connect(ctx context.Context) error {
	logger := shared.GetTunnelLogger("client.connection", c.TunnelID)
	logger.Debug().Msg("starting connection process")

	connectionStart := time.Now()

	u, err := url.Parse(c.ServerURL)
	if err != nil {
		logger.Error().Err(err).Msg("failed to parse server url")
		return err
	}

	wsScheme := "ws"
	if u.Scheme == "https" {
		wsScheme = "wss"
	}

	u.Scheme = wsScheme
	u.RawQuery = "id=" + c.TunnelID
	wsURL := u.String()

	logger.Debug().Str("websocket_url", wsURL).Msg("constructed websocket URL")

	dialer := *websocket.DefaultDialer
	dialer.HandshakeTimeout = 15 * time.Second
	dialer.EnableCompression = true
	dialer.ReadBufferSize = 65536
	dialer.WriteBufferSize = 65536

	logger.Debug().
		Dur("handshake_timeout", dialer.HandshakeTimeout).
		Int("read_buffer_size", dialer.ReadBufferSize).
		Int("write_buffer_size", dialer.WriteBufferSize).
		Bool("compression_enabled", dialer.EnableCompression).
		Msg("websocket dialer configured")

	dialStart := time.Now()
	conn, resp, err := dialer.DialContext(ctx, wsURL, nil)
	dialDuration := time.Since(dialStart)

	if err != nil {
		logger.Error().Err(err).
			Str("websocket_url", wsURL).
			Dur("dial_duration", dialDuration).
			Msg("websocket connection failed")
		if resp != nil {
			logger.Error().Int("http_status", resp.StatusCode).Msg("websocket upgrade response status")
		}
		return err
	}

	totalConnectTime := time.Since(connectionStart)
	logger.Info().
		Dur("dial_time", dialDuration).
		Dur("total_connect_time", totalConnectTime).
		Msg("websocket connection established")

	logger.Debug().
		Int("http_status", resp.StatusCode).
		Msg("websocket upgrade successful")

	c.Conn = conn
	c.connMu.Lock()
	c.lastPong = time.Now()
	c.connMu.Unlock()
	return nil
}
