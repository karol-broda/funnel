package server

import (
	"fmt"
	"sync"
	"testing"

	"github.com/karol-broda/funnel/shared"
)

func TestServerTunnelManagement(t *testing.T) {
	server := NewServer()

	t.Run("add tunnel", func(t *testing.T) {
		tunnelID := "test-tunnel-123"
		tunnel := server.AddTunnel(tunnelID, nil, nil)
		defer server.RemoveTunnel(tunnelID)

		if tunnel == nil {
			t.Error("expected tunnel to be created")
			return
		}

		if tunnel.ID != tunnelID {
			t.Errorf("expected tunnel ID %s, got %s", tunnelID, tunnel.ID)
		}

		if tunnel.conn != nil {
			t.Error("expected tunnel connection to be nil for this test")
		}

		if tunnel.ResponseChannels == nil {
			t.Error("response channels not initialized")
		}

		if tunnel.createdAt.IsZero() {
			t.Error("created timestamp not set")
		}
	})

	t.Run("get tunnel", func(t *testing.T) {
		tunnelID := "test-tunnel-get"
		server.AddTunnel(tunnelID, nil, nil)
		defer server.RemoveTunnel(tunnelID)

		tunnel, exists := server.GetTunnel(tunnelID)
		if !exists {
			t.Error("expected tunnel to exist")
		}

		if tunnel.ID != tunnelID {
			t.Errorf("expected tunnel ID %s, got %s", tunnelID, tunnel.ID)
		}

		_, exists = server.GetTunnel("non-existent")
		if exists {
			t.Error("non-existent tunnel should not exist")
		}
	})

	t.Run("tunnel exists", func(t *testing.T) {
		tunnelID := "test-tunnel-exists"

		if server.TunnelExists(tunnelID) {
			t.Error("tunnel should not exist before creation")
		}

		server.AddTunnel(tunnelID, nil, nil)
		defer server.RemoveTunnel(tunnelID)

		if !server.TunnelExists(tunnelID) {
			t.Error("tunnel should exist after creation")
		}
	})

	t.Run("remove tunnel", func(t *testing.T) {
		tunnelID := "test-tunnel-remove"
		server.AddTunnel(tunnelID, nil, nil)

		if !server.TunnelExists(tunnelID) {
			t.Error("tunnel should exist before removal")
		}

		server.RemoveTunnel(tunnelID)

		if server.TunnelExists(tunnelID) {
			t.Error("tunnel should not exist after removal")
		}

		server.RemoveTunnel("non-existent-tunnel")
	})
}

func TestTunnelConcurrency(t *testing.T) {
	server := NewServer()
	numTunnels := 100

	t.Run("concurrent add", func(t *testing.T) {
		var wg sync.WaitGroup
		for i := 0; i < numTunnels; i++ {
			wg.Add(1)
			go func(i int) {
				defer wg.Done()
				tunnelID := fmt.Sprintf("concurrent-tunnel-%d", i)
				server.AddTunnel(tunnelID, nil, nil)
			}(i)
		}
		wg.Wait()

		server.TunnelsMu.RLock()
		count := len(server.Tunnels)
		server.TunnelsMu.RUnlock()

		if count != numTunnels {
			t.Errorf("expected %d tunnels, got %d", numTunnels, count)
		}

		for i := 0; i < numTunnels; i++ {
			tunnelID := fmt.Sprintf("concurrent-tunnel-%d", i)
			server.RemoveTunnel(tunnelID)
		}
	})

	t.Run("concurrent access", func(t *testing.T) {
		var wg sync.WaitGroup
		for i := 0; i < numTunnels; i++ {
			tunnelID := fmt.Sprintf("concurrent-tunnel-%d", i)
			server.AddTunnel(tunnelID, nil, nil)
			defer server.RemoveTunnel(tunnelID)
		}

		for i := 0; i < numTunnels*2; i++ {
			wg.Add(1)
			go func(i int) {
				defer wg.Done()
				tunnelID := fmt.Sprintf("concurrent-tunnel-%d", i%numTunnels)
				_, exists := server.GetTunnel(tunnelID)
				if !exists {
					t.Errorf("tunnel %s should exist", tunnelID)
				}
			}(i)
		}
		wg.Wait()
	})

	t.Run("concurrent remove", func(t *testing.T) {
		var wg sync.WaitGroup
		for i := 0; i < numTunnels; i++ {
			tunnelID := fmt.Sprintf("concurrent-tunnel-%d", i)
			server.AddTunnel(tunnelID, nil, nil)
		}

		for i := 0; i < numTunnels; i++ {
			wg.Add(1)
			go func(i int) {
				defer wg.Done()
				tunnelID := fmt.Sprintf("concurrent-tunnel-%d", i)
				server.RemoveTunnel(tunnelID)
			}(i)
		}
		wg.Wait()

		server.TunnelsMu.RLock()
		count := len(server.Tunnels)
		server.TunnelsMu.RUnlock()
		if count != 0 {
			t.Errorf("expected 0 tunnels, got %d", count)
		}
	})
}

func TestTunnelMessageHandling(t *testing.T) {
	server := NewServer()

	t.Run("send message to nil connection tunnel", func(t *testing.T) {
		tunnelID := "test-send-receive"
		tunnel := server.AddTunnel(tunnelID, nil, nil)
		defer server.RemoveTunnel(tunnelID)

		testMsg := &shared.Message{
			Type:      "test",
			RequestID: "test-123",
			TunnelID:  tunnelID,
		}
		err := tunnel.SendMessage(testMsg)
		if err == nil {
			t.Error("expected error when sending to a nil connection tunnel, but got nil")
		}
	})
}

func TestTunnelResponseChannels(t *testing.T) {
	server := NewServer()
	tunnel := server.AddTunnel("test-response-channels", nil, nil)
	defer server.RemoveTunnel(tunnel.ID)

	requestID := "test-req-1"
	ch := tunnel.registerResponseChannel(requestID)
	if ch == nil {
		t.Fatal("response channel should not be nil")
	}

	tunnel.ResponseMu.RLock()
	_, exists := tunnel.ResponseChannels[requestID]
	tunnel.ResponseMu.RUnlock()
	if !exists {
		t.Fatal("response channel should be registered")
	}

	tunnel.unregisterResponseChannel(requestID)

	tunnel.ResponseMu.RLock()
	_, exists = tunnel.ResponseChannels[requestID]
	tunnel.ResponseMu.RUnlock()
	if exists {
		t.Fatal("response channel should be unregistered")
	}
}

func BenchmarkTunnelCreation(b *testing.B) {
	server := NewServer()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		tunnelID := fmt.Sprintf("bench-create-%d", i)
		tunnel := server.AddTunnel(tunnelID, nil, nil)
		server.RemoveTunnel(tunnel.ID)
	}
}

func BenchmarkTunnelLookup(b *testing.B) {
	server := NewServer()
	tunnel := server.AddTunnel("bench-lookup", nil, nil)
	defer server.RemoveTunnel(tunnel.ID)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		server.GetTunnel(tunnel.ID)
	}
}
