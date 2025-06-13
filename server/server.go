package server

import (
	"net/http"
	"sync"

	"tunneling/shared"

	"github.com/gorilla/websocket"
)

type Server struct {
	Tunnels   map[string]*Tunnel
	TunnelsMu sync.RWMutex
	Upgrader  websocket.Upgrader
	router    RouterInterface
}

type RouterInterface interface {
	InvalidateCache(tunnelID string)
}

func NewServer() *Server {
	logger := shared.GetLogger("server")

	server := &Server{
		Tunnels: make(map[string]*Tunnel),
		Upgrader: websocket.Upgrader{
			CheckOrigin: func(r *http.Request) bool {
				return true
			},
			ReadBufferSize:  1024 * 64,
			WriteBufferSize: 1024 * 64,
		},
	}

	logger.Info().
		Int("read_buffer_size", server.Upgrader.ReadBufferSize).
		Int("write_buffer_size", server.Upgrader.WriteBufferSize).
		Bool("check_origin_disabled", true).
		Msg("server initialized")

	return server
}

func (s *Server) SetRouter(router RouterInterface) {
	logger := shared.GetLogger("server")

	s.router = router
	logger.Debug().Msg("router configured for server")
}
