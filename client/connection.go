package client

import (
	"fmt"
	"net/url"
	"time"

	"github.com/karol-broda/funnel/shared"

	"github.com/gorilla/websocket"
)

func (c *Client) connect() error {
	logger := shared.GetTunnelLogger("client.connection", c.TunnelID)

	connectStart := time.Now()
	logger.Debug().Str("server_url", c.ServerURL).Msg("starting connection process")

	u, err := url.Parse(c.ServerURL)
	if err != nil {
		logger.Error().Err(err).Str("server_url", c.ServerURL).Msg("failed to parse server URL")
		return err
	}

	wsScheme := "ws"
	if u.Scheme == "https" {
		wsScheme = "wss"
		logger.Debug().Msg("using secure websocket connection (wss)")
	} else {
		logger.Debug().Msg("using insecure websocket connection (ws)")
	}

	wsURL := fmt.Sprintf("%s://%s/?id=%s", wsScheme, u.Host, c.TunnelID)
	logger.Debug().Str("websocket_url", wsURL).Msg("constructed websocket URL")

	dialer := websocket.Dialer{
		HandshakeTimeout: 10 * time.Second,
		ReadBufferSize:   1024 * 64,
		WriteBufferSize:  1024 * 64,
	}

	logger.Debug().
		Dur("handshake_timeout", dialer.HandshakeTimeout).
		Int("read_buffer_size", dialer.ReadBufferSize).
		Int("write_buffer_size", dialer.WriteBufferSize).
		Msg("websocket dialer configured")

	dialStart := time.Now()
	conn, resp, err := dialer.Dial(wsURL, nil)
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

	connectDuration := time.Since(connectStart)
	logger.Info().
		Dur("total_connect_time", connectDuration).
		Dur("dial_time", dialDuration).
		Msg("websocket connection established")

	if resp != nil {
		logger.Debug().Int("http_status", resp.StatusCode).Msg("websocket upgrade successful")
	}

	c.Conn = conn
	return nil
}
