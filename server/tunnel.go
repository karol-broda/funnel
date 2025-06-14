package server

import (
	"fmt"
	"sync"
	"time"

	"github.com/karol-broda/funnel/shared"

	"github.com/gorilla/websocket"
)

type Tunnel struct {
	ID               string
	conn             *websocket.Conn
	incomingMessages chan *shared.Message
	outgoingMessages chan *shared.Message
	server           *Server
	closeOnce        sync.Once
	runOnce          sync.Once
	removed          bool
	removedMu        sync.Mutex

	ResponseChannels map[string]chan *shared.Message
	ResponseMu       sync.RWMutex

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

func (s *Server) AddTunnel(id string, conn *websocket.Conn, wg *sync.WaitGroup) *Tunnel {
	logger := shared.GetTunnelLogger("server.tunnel", id)

	tunnel := &Tunnel{
		ID:               id,
		conn:             conn,
		ResponseChannels: make(map[string]chan *shared.Message),
		incomingMessages: make(chan *shared.Message, 100),
		outgoingMessages: make(chan *shared.Message, 100),
		server:           s,
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

	return tunnel
}

func (s *Server) RemoveTunnel(id string) {
	s.TunnelsMu.Lock()
	tunnel, exists := s.Tunnels[id]
	delete(s.Tunnels, id)
	remainingTunnels := len(s.Tunnels)
	s.TunnelsMu.Unlock()

	if exists {
		tunnel.removedMu.Lock()
		tunnel.removed = true
		tunnel.removedMu.Unlock()

		tunnel.closeConnection()

		if s.router != nil {
			s.router.InvalidateCache(id)
		}

		logger := shared.GetTunnelLogger("server.tunnel", id)
		lifetime := time.Since(tunnel.createdAt)
		logger.Info().
			Int("remaining_tunnels", remainingTunnels).
			Dur("tunnel_lifetime", lifetime).
			Int64("messages_received", tunnel.messagesReceived).
			Int64("messages_sent", tunnel.messagesSent).
			Int64("bytes_received", tunnel.bytesReceived).
			Int64("bytes_sent", tunnel.bytesSent).
			Msg("tunnel removed from server")
	} else {
		logger := shared.GetTunnelLogger("server.tunnel", id)
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
	defer func() {
		if r := recover(); r != nil {
			logger := shared.GetTunnelLogger("server.tunnel", t.ID)
			logger.Error().Msgf("recovered in readmessages: %v", r)
		}
		t.closeConnection()
	}()

	logger := shared.GetTunnelLogger("server.tunnel", t.ID)
	logger.Debug().Msg("starting message reader goroutine")

	if t.conn == nil {
		logger.Error().Msg("tunnel connection is nil, cannot read messages")
		return
	}

	for {
		var msg shared.Message

		readStart := time.Now()
		if t.conn == nil {
			logger.Error().Msg("tunnel connection became nil during read")
			return
		}
		err := t.conn.ReadJSON(&msg)
		readDuration := time.Since(readStart)

		if err != nil {
			logger.Error().Err(err).
				Dur("read_duration", readDuration).
				Int64("total_messages_received", t.messagesReceived).
				Msg("websocket read failed")
			t.closeConnection()
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
		default:
			logger.Warn().
				Str("message_type", msg.Type).
				Int("queue_capacity", cap(t.incomingMessages)).
				Msg("message channel full, dropping message")
		}
	}
}

func (t *Tunnel) writeMessages() {
	defer func() {
		if r := recover(); r != nil {
			logger := shared.GetTunnelLogger("server.tunnel", t.ID)
			logger.Error().Msgf("recovered in writemessages: %v", r)
		}
		t.closeConnection()
	}()

	logger := shared.GetTunnelLogger("server.tunnel", t.ID)
	logger.Debug().Msg("starting message writer goroutine")

	if t.conn == nil {
		logger.Error().Msg("tunnel connection is nil, cannot write messages")
		return
	}

	for {
		select {
		case msg := <-t.outgoingMessages:
			if msg == nil {
				logger.Debug().Msg("received nil message, writer stopping")
				return
			}

			writeStart := time.Now()
			if t.conn == nil {
				logger.Error().Msg("tunnel connection became nil during write")
				return
			}
			err := t.conn.WriteJSON(msg)
			writeDuration := time.Since(writeStart)

			if err != nil {
				logger.Error().Err(err).
					Str("message_type", msg.Type).
					Str("request_id", msg.RequestID).
					Dur("write_duration", writeDuration).
					Msg("websocket write failed")
				t.closeConnection()
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
	if t.conn == nil {
		return fmt.Errorf("tunnel connection is nil")
	}

	if t.outgoingMessages == nil {
		return fmt.Errorf("tunnel is closed, dropping message")
	}

	logger := shared.GetTunnelLogger("server.tunnel", t.ID)
	if msg.TunnelID != t.ID {
		logger.Error().
			Str("expected_tunnel_id", t.ID).
			Str("message_tunnel_id", msg.TunnelID).
			Str("request_id", msg.RequestID).
			Msg("message dropped: tunnel id mismatch")
		return fmt.Errorf("message dropped: tunnel id mismatch")
	}

	select {
	case t.outgoingMessages <- msg:
		logger.Debug().
			Str("message_type", msg.Type).
			Str("request_id", msg.RequestID).
			Int("queue_len", len(t.outgoingMessages)).
			Msg("message queued for sending")
		return nil
	default:
		logger.Error().
			Str("message_type", msg.Type).
			Str("request_id", msg.RequestID).
			Int("queue_capacity", cap(t.outgoingMessages)).
			Msg("outgoing message queue full, dropping message")
		return fmt.Errorf("outgoing message queue full")
	}
}

func (t *Tunnel) routeMessages() {
	defer func() {
		if r := recover(); r != nil {
			logger := shared.GetTunnelLogger("server.tunnel", t.ID)
			logger.Error().Msgf("recovered in routemessages: %v", r)
		}
		channelCount := 0
		t.ResponseMu.RLock()
		for _, ch := range t.ResponseChannels {
			close(ch)
			channelCount++
		}
		t.ResponseMu.RUnlock()
		if channelCount > 0 {
			logger := shared.GetTunnelLogger("server.tunnel", t.ID)
			logger.Warn().Int("closed_channels", channelCount).Msg("cleaned up pending response channels")
		}
		logger := shared.GetTunnelLogger("server.tunnel", t.ID)
		logger.Debug().Msg("message router stopped")
	}()

	logger := shared.GetTunnelLogger("server.tunnel", t.ID)
	logger.Debug().Msg("starting message router goroutine")

	for {
		select {
		case msg := <-t.incomingMessages:
			if msg == nil {
				logger.Debug().Msg("received nil message, router stopping")
				return
			}

			switch msg.Type {
			case "response":
				t.ResponseMu.RLock()
				if respChan, ok := t.ResponseChannels[msg.RequestID]; ok {
					select {
					case respChan <- msg:
					default:
						// Non-blocking send
					}
				}
				t.ResponseMu.RUnlock()
			case "ping":
				t.SendMessage(&shared.Message{Type: "pong"})
			default:
				logger.Debug().
					Str("message_type", msg.Type).
					Str("request_id", msg.RequestID).
					Msg("unhandled message type in router")
			}
		}
	}
}

func (t *Tunnel) registerResponseChannel(requestID string) chan *shared.Message {
	respChan := make(chan *shared.Message, 1)
	t.ResponseMu.Lock()
	t.ResponseChannels[requestID] = respChan
	t.ResponseMu.Unlock()
	logger := shared.GetTunnelLogger("server.tunnel", t.ID)
	logger.Debug().
		Str("request_id", requestID).
		Msg("response channel registered")
	return respChan
}

func (t *Tunnel) unregisterResponseChannel(requestID string) {
	t.ResponseMu.Lock()
	if ch, ok := t.ResponseChannels[requestID]; ok {
		close(ch)
		delete(t.ResponseChannels, requestID)
	}
	t.ResponseMu.Unlock()
	logger := shared.GetRequestLogger("server.tunnel", t.ID, requestID)
	logger.Debug().
		Str("request_id", requestID).
		Msg("response channel unregistered")
}

func (t *Tunnel) Run() {
	t.runOnce.Do(func() {
		defer func() {
			if r := recover(); r != nil {
				logger := shared.GetTunnelLogger("server.tunnel", t.ID)
				logger.Error().Msgf("recovered in run: %v", r)
			}
			t.closeConnection()
			t.removedMu.Lock()
			if !t.removed && t.server != nil {
				t.removed = true
				t.removedMu.Unlock()
				t.server.RemoveTunnel(t.ID)
			} else {
				t.removedMu.Unlock()
			}
		}()

		logger := shared.GetTunnelLogger("server.tunnel", t.ID)
		logger.Debug().Msg("tunnel is now running")

		if t.conn == nil {
			logger.Warn().Msg("tunnel has a nil connection, not starting goroutines")
			return
		}

		done := make(chan struct{})
		var wg sync.WaitGroup
		wg.Add(3)

		go func() {
			defer wg.Done()
			t.readMessages()
		}()

		go func() {
			defer wg.Done()
			t.writeMessages()
		}()

		go func() {
			defer wg.Done()
			t.routeMessages()
		}()

		// Wait for all goroutines to complete
		go func() {
			wg.Wait()
			close(done)
		}()

		// Block until all goroutines are done
		<-done
		logger.Debug().Msg("tunnel has stopped")
	})
}

func (t *Tunnel) closeConnection() {
	t.closeOnce.Do(func() {
		if t.conn != nil {
			t.conn.Close()
			t.conn = nil
		}
		if t.outgoingMessages != nil {
			close(t.outgoingMessages)
			t.outgoingMessages = nil
		}
		logger := shared.GetTunnelLogger("server.tunnel", t.ID)
		logger.Debug().Msg("tunnel has stopped")
	})
}
