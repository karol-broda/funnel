package server

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/karol-broda/go-tunnel-proxy/shared"

	"github.com/gorilla/websocket"
)

func TestServerTunnelManagement(t *testing.T) {
	server := NewServer()

	// test tunnel creation
	t.Run("add tunnel", func(t *testing.T) {
		// create a mock websocket connection
		mockConn := &websocket.Conn{}
		tunnelID := "test-tunnel-123"

		tunnel := server.AddTunnel(tunnelID, mockConn)

		if tunnel == nil {
			t.Error("expected tunnel to be created")
			return
		}

		if tunnel.ID != tunnelID {
			t.Errorf("expected tunnel ID %s, got %s", tunnelID, tunnel.ID)
		}

		if tunnel.Conn != mockConn {
			t.Error("tunnel connection mismatch")
		}

		if tunnel.ResponseChannels == nil {
			t.Error("response channels not initialized")
		}

		if tunnel.createdAt.IsZero() {
			t.Error("created timestamp not set")
		}
	})

	// test tunnel retrieval
	t.Run("get tunnel", func(t *testing.T) {
		mockConn := &websocket.Conn{}
		tunnelID := "test-tunnel-get"

		server.AddTunnel(tunnelID, mockConn)

		tunnel, exists := server.GetTunnel(tunnelID)
		if !exists {
			t.Error("tunnel should exist")
		}

		if tunnel.ID != tunnelID {
			t.Errorf("expected tunnel ID %s, got %s", tunnelID, tunnel.ID)
		}

		// test non-existent tunnel
		_, exists = server.GetTunnel("non-existent")
		if exists {
			t.Error("non-existent tunnel should not exist")
		}
	})

	// test tunnel existence check
	t.Run("tunnel exists", func(t *testing.T) {
		mockConn := &websocket.Conn{}
		tunnelID := "test-tunnel-exists"

		if server.TunnelExists(tunnelID) {
			t.Error("tunnel should not exist before creation")
		}

		server.AddTunnel(tunnelID, mockConn)

		if !server.TunnelExists(tunnelID) {
			t.Error("tunnel should exist after creation")
		}
	})

	// test tunnel removal
	t.Run("remove tunnel", func(t *testing.T) {
		mockConn := &websocket.Conn{}
		tunnelID := "test-tunnel-remove"

		server.AddTunnel(tunnelID, mockConn)

		if !server.TunnelExists(tunnelID) {
			t.Error("tunnel should exist before removal")
		}

		server.RemoveTunnel(tunnelID)

		if server.TunnelExists(tunnelID) {
			t.Error("tunnel should not exist after removal")
		}

		// test removing non-existent tunnel (should not panic)
		server.RemoveTunnel("non-existent-tunnel")
	})
}

func TestTunnelConcurrency(t *testing.T) {
	server := NewServer()
	numGoroutines := 100
	tunnelPrefix := "concurrent-tunnel-"

	var wg sync.WaitGroup

	// test concurrent tunnel creation
	t.Run("concurrent add", func(t *testing.T) {
		wg.Add(numGoroutines)

		for i := 0; i < numGoroutines; i++ {
			go func(id int) {
				defer wg.Done()
				mockConn := &websocket.Conn{}
				tunnelID := tunnelPrefix + string(rune(id))
				server.AddTunnel(tunnelID, mockConn)
			}(i)
		}

		wg.Wait()

		// verify all tunnels were created
		server.TunnelsMu.RLock()
		count := len(server.Tunnels)
		server.TunnelsMu.RUnlock()

		if count < numGoroutines {
			t.Errorf("expected at least %d tunnels, got %d", numGoroutines, count)
		}
	})

	// test concurrent tunnel access
	t.Run("concurrent access", func(t *testing.T) {
		wg.Add(numGoroutines)

		for i := 0; i < numGoroutines; i++ {
			go func(id int) {
				defer wg.Done()
				tunnelID := tunnelPrefix + string(rune(id))

				// try to get tunnel
				_, exists := server.GetTunnel(tunnelID)
				if !exists {
					t.Errorf("tunnel %s should exist", tunnelID)
				}

				// check existence
				if !server.TunnelExists(tunnelID) {
					t.Errorf("tunnel %s should exist", tunnelID)
				}
			}(i)
		}

		wg.Wait()
	})

	// test concurrent tunnel removal
	t.Run("concurrent remove", func(t *testing.T) {
		wg.Add(numGoroutines)

		for i := 0; i < numGoroutines; i++ {
			go func(id int) {
				defer wg.Done()
				tunnelID := tunnelPrefix + string(rune(id))
				server.RemoveTunnel(tunnelID)
			}(i)
		}

		wg.Wait()

		// verify tunnels were removed
		server.TunnelsMu.RLock()
		count := len(server.Tunnels)
		server.TunnelsMu.RUnlock()

		if count > 0 {
			t.Errorf("expected 0 tunnels after removal, got %d", count)
		}
	})
}

func TestTunnelMessageHandling(t *testing.T) {
	server := NewServer()

	// create a test HTTP server for websocket connections
	upgrader := websocket.Upgrader{
		CheckOrigin: func(r *http.Request) bool {
			return true
		},
	}

	testServer := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		conn, err := upgrader.Upgrade(w, r, nil)
		if err != nil {
			t.Errorf("websocket upgrade failed: %v", err)
			return
		}
		defer conn.Close()

		// simulate message handling
		for {
			var msg shared.Message
			if err := conn.ReadJSON(&msg); err != nil {
				break
			}

			// echo the message back
			if err := conn.WriteJSON(&msg); err != nil {
				break
			}
		}
	}))
	defer testServer.Close()

	t.Run("send message", func(t *testing.T) {
		// create websocket connection
		wsURL := "ws" + strings.TrimPrefix(testServer.URL, "http")
		conn, _, err := websocket.DefaultDialer.Dial(wsURL, nil)
		if err != nil {
			t.Fatalf("failed to connect: %v", err)
		}
		defer conn.Close()

		tunnel := server.AddTunnel("test-send-message", conn)
		defer server.RemoveTunnel("test-send-message")

		// test sending a message
		testMsg := &shared.Message{
			Type:      "test",
			RequestID: "test-123",
			Method:    "GET",
			Path:      "/test",
		}

		err = tunnel.SendMessage(testMsg)
		if err != nil {
			t.Errorf("failed to send message: %v", err)
		}

		// verify message was sent by reading it back
		var receivedMsg shared.Message
		err = conn.ReadJSON(&receivedMsg)
		if err != nil {
			t.Errorf("failed to read message: %v", err)
		}

		if receivedMsg.Type != testMsg.Type {
			t.Errorf("expected message type %s, got %s", testMsg.Type, receivedMsg.Type)
		}
	})
}

func TestTunnelStats(t *testing.T) {
	server := NewServer()
	mockConn := &websocket.Conn{}
	tunnelID := "test-stats"

	tunnel := server.AddTunnel(tunnelID, mockConn)
	defer server.RemoveTunnel(tunnelID)

	// test initial stats
	if tunnel.messagesReceived != 0 {
		t.Errorf("expected 0 messages received, got %d", tunnel.messagesReceived)
	}

	if tunnel.messagesSent != 0 {
		t.Errorf("expected 0 messages sent, got %d", tunnel.messagesSent)
	}

	if tunnel.bytesReceived != 0 {
		t.Errorf("expected 0 bytes received, got %d", tunnel.bytesReceived)
	}

	if tunnel.bytesSent != 0 {
		t.Errorf("expected 0 bytes sent, got %d", tunnel.bytesSent)
	}

	// test lifetime
	lifetime := time.Since(tunnel.createdAt)
	if lifetime < 0 {
		t.Error("tunnel lifetime should be positive")
	}
}

func TestTunnelResponseChannels(t *testing.T) {
	server := NewServer()
	mockConn := &websocket.Conn{}
	tunnelID := "test-response-channels"

	tunnel := server.AddTunnel(tunnelID, mockConn)
	defer server.RemoveTunnel(tunnelID)

	requestID := "test-request-123"

	// test registering response channel
	ch := tunnel.registerResponseChannel(requestID)
	if ch == nil {
		t.Error("response channel should not be nil")
	}

	// verify channel is registered
	tunnel.ResponseMu.RLock()
	_, exists := tunnel.ResponseChannels[requestID]
	tunnel.ResponseMu.RUnlock()

	if !exists {
		t.Error("response channel should be registered")
	}

	// test unregistering response channel
	tunnel.unregisterResponseChannel(requestID)

	// verify channel is unregistered
	tunnel.ResponseMu.RLock()
	_, exists = tunnel.ResponseChannels[requestID]
	tunnel.ResponseMu.RUnlock()

	if exists {
		t.Error("response channel should be unregistered")
	}
}

func TestTunnelMemoryLeaks(t *testing.T) {
	server := NewServer()
	numTunnels := 100

	// create and remove many tunnels
	for i := 0; i < numTunnels; i++ {
		mockConn := &websocket.Conn{}
		tunnelID := "leak-test-" + string(rune(i))

		tunnel := server.AddTunnel(tunnelID, mockConn)

		// register some response channels
		for j := 0; j < 10; j++ {
			requestID := "req-" + string(rune(j))
			tunnel.registerResponseChannel(requestID)
		}

		server.RemoveTunnel(tunnelID)
	}

	// verify all tunnels are cleaned up
	server.TunnelsMu.RLock()
	count := len(server.Tunnels)
	server.TunnelsMu.RUnlock()

	if count != 0 {
		t.Errorf("expected 0 tunnels after cleanup, got %d", count)
	}
}

// benchmark tests
func BenchmarkTunnelCreation(b *testing.B) {
	server := NewServer()
	mockConn := &websocket.Conn{}

	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		tunnelID := "bench-" + string(rune(i))
		tunnel := server.AddTunnel(tunnelID, mockConn)
		server.RemoveTunnel(tunnel.ID)
	}
}

func BenchmarkTunnelLookup(b *testing.B) {
	server := NewServer()
	mockConn := &websocket.Conn{}
	tunnelID := "bench-lookup"

	server.AddTunnel(tunnelID, mockConn)
	defer server.RemoveTunnel(tunnelID)

	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		_, _ = server.GetTunnel(tunnelID)
	}
}

func BenchmarkTunnelExists(b *testing.B) {
	server := NewServer()
	mockConn := &websocket.Conn{}
	tunnelID := "bench-exists"

	server.AddTunnel(tunnelID, mockConn)
	defer server.RemoveTunnel(tunnelID)

	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		_ = server.TunnelExists(tunnelID)
	}
}
