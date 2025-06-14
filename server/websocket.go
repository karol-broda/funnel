package server

import (
	"net/http"
	"time"

	"github.com/karol-broda/go-tunnel-proxy/shared"
)

func (s *Server) HandleWebSocket(w http.ResponseWriter, r *http.Request) {
	logger := shared.GetLogger("server.websocket")

	logger.Debug().
		Str("remote_addr", r.RemoteAddr).
		Str("user_agent", r.UserAgent()).
		Str("origin", r.Header.Get("Origin")).
		Msg("websocket connection attempt")

	tunnelID := r.URL.Query().Get("id")
	if tunnelID == "" {
		logger.Warn().
			Str("remote_addr", r.RemoteAddr).
			Msg("websocket upgrade rejected - missing tunnel id")
		http.Error(w, "tunnel id required", http.StatusBadRequest)
		return
	}

	tunnelLogger := shared.GetTunnelLogger("server.websocket", tunnelID)

	tunnelLogger.Info().
		Str("remote_addr", r.RemoteAddr).
		Str("user_agent", r.UserAgent()).
		Msg("websocket upgrade requested")

	if s.TunnelExists(tunnelID) {
		tunnelLogger.Warn().
			Str("remote_addr", r.RemoteAddr).
			Msg("websocket upgrade rejected - tunnel id already in use")
		http.Error(w, "tunnel id already in use", http.StatusConflict)
		return
	}

	upgradeStart := time.Now()
	conn, err := s.Upgrader.Upgrade(w, r, nil)
	upgradeDuration := time.Since(upgradeStart)

	if err != nil {
		tunnelLogger.Error().Err(err).
			Dur("upgrade_duration", upgradeDuration).
			Str("remote_addr", r.RemoteAddr).
			Msg("websocket upgrade failed")
		return
	}

	tunnelLogger.Info().
		Dur("upgrade_duration", upgradeDuration).
		Str("remote_addr", r.RemoteAddr).
		Msg("websocket upgrade successful")

	tunnel := s.AddTunnel(tunnelID, conn)
	tunnelLogger.Info().Msg("tunnel connected via websocket")

	defer func() {
		s.RemoveTunnel(tunnelID)
		conn.Close()
		tunnelLogger.Info().Msg("tunnel disconnected")
	}()

	s.setupWebSocketConnection(tunnel)

	connectionStart := time.Now()
	tunnelLogger.Debug().Msg("starting connection monitoring loop")

	for {
		if !s.TunnelExists(tunnelID) {
			connectionDuration := time.Since(connectionStart)
			tunnelLogger.Info().
				Dur("connection_duration", connectionDuration).
				Msg("tunnel connection ended")
			break
		}
		time.Sleep(1 * time.Second)
	}
}

func (s *Server) setupWebSocketConnection(tunnel *Tunnel) {
	logger := shared.GetTunnelLogger("server.websocket", tunnel.ID)

	readDeadline := 300 * time.Second
	if err := tunnel.Conn.SetReadDeadline(time.Now().Add(readDeadline)); err != nil {
		logger.Error().Err(err).Msg("failed to set initial read deadline")
		return
	}

	logger.Debug().
		Dur("read_deadline", readDeadline).
		Msg("websocket read deadline configured")

	tunnel.Conn.SetPongHandler(func(string) error {
		if err := tunnel.Conn.SetReadDeadline(time.Now().Add(readDeadline)); err != nil {
			logger.Error().Err(err).Msg("failed to extend read deadline on pong")
			return err
		}
		logger.Debug().Msg("pong received, read deadline extended")
		return nil
	})

	logger.Debug().Msg("websocket connection setup completed")
}
