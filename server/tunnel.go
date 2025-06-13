package server

import (
	"fmt"
	"sync"
	"time"

	"tunneling/shared"

	"github.com/gorilla/websocket"
)

type Tunnel struct {
	ID   string
	Conn *websocket.Conn

	ReadMu  sync.Mutex
	WriteMu sync.Mutex

	ResponseChannels map[string]chan *shared.Message
	ResponseMu       sync.RWMutex

	incomingMessages chan *shared.Message
	outgoingMessages chan *shared.Message
	stopRouter       chan struct{}

	createdAt        time.Time
	messagesReceived int64
	messagesSent     int64
	bytesReceived    int64
	bytesSent        int64
}

func (s *Server) GetTunnel(id string) (*Tunnel, bool) {
	s.TunnelsMu.RLock()
	defer s.TunnelsMu.RUnlock()
	tunnel, exists := s.Tunnels[id]
	return tunnel, exists
}

func (s *Server) AddTunnel(id string, conn *websocket.Conn) *Tunnel {
	logger := shared.GetTunnelLogger("server.tunnel", id)

	tunnel := &Tunnel{
		ID:               id,
		Conn:             conn,
		ResponseChannels: make(map[string]chan *shared.Message),
		incomingMessages: make(chan *shared.Message, 100),
		outgoingMessages: make(chan *shared.Message, 100),
		stopRouter:       make(chan struct{}),
		createdAt:        time.Now(),
	}

	s.TunnelsMu.Lock()
	tunnelCount := len(s.Tunnels)
	s.Tunnels[id] = tunnel
	s.TunnelsMu.Unlock()

	logger.Info().
		Int("total_tunnels", tunnelCount+1).
		Int("incoming_buffer_size", cap(tunnel.incomingMessages)).
		Int("outgoing_buffer_size", cap(tunnel.outgoingMessages)).
		Msg("tunnel added to server")

	go tunnel.readMessages()
	go tunnel.writeMessages()
	go tunnel.routeMessages()

	return tunnel
}

func (s *Server) RemoveTunnel(id string) {
	logger := shared.GetTunnelLogger("server.tunnel", id)

	s.TunnelsMu.Lock()
	tunnel, exists := s.Tunnels[id]
	delete(s.Tunnels, id)
	remainingTunnels := len(s.Tunnels)
	s.TunnelsMu.Unlock()

	if exists {
		lifetime := time.Since(tunnel.createdAt)

		close(tunnel.stopRouter)
		close(tunnel.incomingMessages)
		close(tunnel.outgoingMessages)

		if s.router != nil {
			s.router.InvalidateCache(id)
		}

		logger.Info().
			Int("remaining_tunnels", remainingTunnels).
			Dur("tunnel_lifetime", lifetime).
			Int64("messages_received", tunnel.messagesReceived).
			Int64("messages_sent", tunnel.messagesSent).
			Int64("bytes_received", tunnel.bytesReceived).
			Int64("bytes_sent", tunnel.bytesSent).
			Msg("tunnel removed from server")
	} else {
		logger.Warn().Msg("attempted to remove non-existent tunnel")
	}
}

func (s *Server) TunnelExists(id string) bool {
	s.TunnelsMu.RLock()
	defer s.TunnelsMu.RUnlock()
	_, exists := s.Tunnels[id]
	return exists
}

func (t *Tunnel) readMessages() {
	logger := shared.GetTunnelLogger("server.tunnel", t.ID)

	logger.Debug().Msg("starting message reader goroutine")

	for {
		select {
		case <-t.stopRouter:
			logger.Debug().Msg("message reader stopping")
			return
		default:
			var msg shared.Message

			readStart := time.Now()
			t.ReadMu.Lock()
			err := t.Conn.ReadJSON(&msg)
			t.ReadMu.Unlock()
			readDuration := time.Since(readStart)

			if err != nil {
				logger.Error().Err(err).
					Dur("read_duration", readDuration).
					Int64("total_messages_received", t.messagesReceived).
					Msg("websocket read failed")
				return
			}

			t.messagesReceived++
			messageSize := int64(len(msg.Body))
			t.bytesReceived += messageSize

			logger.Debug().
				Str("message_type", msg.Type).
				Str("request_id", msg.RequestID).
				Int64("message_size", messageSize).
				Dur("read_duration", readDuration).
				Msg("message received from client")

			select {
			case t.incomingMessages <- &msg:
			case <-t.stopRouter:
				logger.Debug().Msg("message reader stopping during message send")
				return
			default:
				logger.Warn().
					Str("message_type", msg.Type).
					Int("queue_capacity", cap(t.incomingMessages)).
					Msg("message channel full, dropping message")
			}
		}
	}
}

func (t *Tunnel) writeMessages() {
	logger := shared.GetTunnelLogger("server.tunnel", t.ID)

	logger.Debug().Msg("starting message writer goroutine")

	for {
		select {
		case <-t.stopRouter:
			logger.Debug().Msg("message writer stopping")
			return
		case msg := <-t.outgoingMessages:
			if msg == nil {
				logger.Debug().Msg("received nil message, writer stopping")
				return
			}

			writeStart := time.Now()
			t.WriteMu.Lock()
			err := t.Conn.WriteJSON(msg)
			t.WriteMu.Unlock()
			writeDuration := time.Since(writeStart)

			if err != nil {
				logger.Error().Err(err).
					Str("message_type", msg.Type).
					Str("request_id", msg.RequestID).
					Dur("write_duration", writeDuration).
					Msg("websocket write failed")
				return
			}

			t.messagesSent++
			messageSize := int64(len(msg.Body))
			t.bytesSent += messageSize

			logger.Debug().
				Str("message_type", msg.Type).
				Str("request_id", msg.RequestID).
				Int64("message_size", messageSize).
				Dur("write_duration", writeDuration).
				Msg("message sent to client")
		}
	}
}

func (t *Tunnel) SendMessage(msg *shared.Message) error {
	logger := shared.GetTunnelLogger("server.tunnel", t.ID)

	select {
	case t.outgoingMessages <- msg:
		logger.Debug().
			Str("message_type", msg.Type).
			Str("request_id", msg.RequestID).
			Msg("message queued for sending")
		return nil
	default:
		queueSize := len(t.outgoingMessages)
		queueCapacity := cap(t.outgoingMessages)
		logger.Error().
			Int("queue_size", queueSize).
			Int("queue_capacity", queueCapacity).
			Str("message_type", msg.Type).
			Str("request_id", msg.RequestID).
			Msg("outgoing message queue full")
		return fmt.Errorf("outgoing message queue full")
	}
}

func (t *Tunnel) routeMessages() {
	logger := shared.GetTunnelLogger("server.tunnel", t.ID)

	logger.Debug().Msg("starting message router goroutine")

	defer func() {
		t.ResponseMu.Lock()
		channelCount := len(t.ResponseChannels)
		for _, ch := range t.ResponseChannels {
			close(ch)
		}
		t.ResponseChannels = make(map[string]chan *shared.Message)
		t.ResponseMu.Unlock()
		if channelCount > 0 {
			logger.Warn().Int("closed_channels", channelCount).Msg("cleaned up pending response channels")
		}
		logger.Debug().Msg("message router stopped")
	}()

	for {
		select {
		case <-t.stopRouter:
			logger.Debug().Msg("message router stopping")
			return
		case msg := <-t.incomingMessages:
			if msg == nil {
				logger.Debug().Msg("received nil message, router stopping")
				return
			}

			if msg.Type == "response" && msg.RequestID != "" {
				reqLogger := shared.GetRequestLogger("server.tunnel", t.ID, msg.RequestID)

				routeStart := time.Now()
				t.ResponseMu.RLock()
				if ch, exists := t.ResponseChannels[msg.RequestID]; exists {
					select {
					case ch <- msg:
						routeDuration := time.Since(routeStart)
						reqLogger.Debug().
							Dur("route_duration", routeDuration).
							Int("status_code", msg.Status).
							Int("response_size", len(msg.Body)).
							Msg("response routed to waiting request")
					default:
						reqLogger.Warn().Msg("response channel full or closed")
					}
				} else {
					reqLogger.Warn().
						Int("active_channels", len(t.ResponseChannels)).
						Msg("no response channel found for request")
				}
				t.ResponseMu.RUnlock()
			} else {
				logger.Debug().
					Str("message_type", msg.Type).
					Str("request_id", msg.RequestID).
					Msg("ignoring non-response message")
			}
		}
	}
}

func (t *Tunnel) registerResponseChannel(requestID string) chan *shared.Message {
	logger := shared.GetTunnelLogger("server.tunnel", t.ID)

	ch := make(chan *shared.Message, 1)
	t.ResponseMu.Lock()
	t.ResponseChannels[requestID] = ch
	channelCount := len(t.ResponseChannels)
	t.ResponseMu.Unlock()

	logger.Debug().
		Str("request_id", requestID).
		Int("active_channels", channelCount).
		Msg("response channel registered")

	return ch
}

func (t *Tunnel) unregisterResponseChannel(requestID string) {
	logger := shared.GetTunnelLogger("server.tunnel", t.ID)

	t.ResponseMu.Lock()
	delete(t.ResponseChannels, requestID)
	channelCount := len(t.ResponseChannels)
	t.ResponseMu.Unlock()

	logger.Debug().
		Str("request_id", requestID).
		Int("remaining_channels", channelCount).
		Msg("response channel unregistered")
}
