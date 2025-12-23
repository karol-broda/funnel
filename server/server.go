package server

import (
	"net/http"
	"sync"

	"github.com/karol-broda/funnel/shared"

	"github.com/gorilla/websocket"
)

type Server struct {
	Tunnels    map[string]*Tunnel
	TunnelsMu  sync.RWMutex
	Upgrader   websocket.Upgrader
	router     RouterInterface
	tokenStore *TokenStore
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

func (s *Server) SetTokenStore(tokenStore *TokenStore) {
	logger := shared.GetLogger("server")

	s.tokenStore = tokenStore
	if tokenStore.IsEnabled() {
		logger.Info().
			Int("active_tokens", tokenStore.Count()).
			Msg("token store configured - authentication enabled")
	} else {
		logger.Warn().Msg("token store not configured - authentication disabled")
	}
}

func (s *Server) GetTokenStore() *TokenStore {
	return s.tokenStore
}

func (s *Server) ValidateToken(token string) (*TokenRecord, bool) {
	if s.tokenStore == nil {
		return nil, true
	}
	return s.tokenStore.Validate(token)
}
