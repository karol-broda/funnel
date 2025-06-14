package server

import (
	"strings"
	"testing"
)

func BenchmarkSubdomainExtraction(b *testing.B) {
	server := NewServer()
	tunnelRouter := NewTunnelRouter(server)
	testHost := "K7mN9pXq.localhost:8080"

	b.Run("TunnelRouter", func(b *testing.B) {
		b.ReportAllocs()
		for i := 0; i < b.N; i++ {
			tunnelRouter.extractSubdomain(testHost)
		}
	})

	b.Run("StringsSplit", func(b *testing.B) {
		b.ReportAllocs()
		for i := 0; i < b.N; i++ {
			host := testHost
			if idx := strings.Index(host, ":"); idx != -1 {
				host = host[:idx]
			}
			parts := strings.Split(host, ".")
			if len(parts) >= 2 {
				_ = parts[0]
			}
		}
	})
}

func BenchmarkCacheEfficiency(b *testing.B) {
	server := NewServer()
	tunnelRouter := NewTunnelRouter(server)

	testHost := "cached-tunnel.localhost:8080"
	tunnelRouter.getSubdomain(testHost)

	b.ResetTimer()
	b.ReportAllocs()

	for i := 0; i < b.N; i++ {
		tunnelRouter.getSubdomain(testHost)
	}
}

func TestTunnelRouterFunctionality(t *testing.T) {
	server := NewServer()
	tunnelRouter := NewTunnelRouter(server)

	tests := []struct {
		host     string
		expected string
	}{
		{"K7mN9pXq.localhost:8080", "K7mN9pXq"},
		{"test-app.example.com", "test-app"},
		{"abc123.localhost", "abc123"},
		{"localhost:8080", ""},
		{"invalid", ""},
		{".localhost", ""},
		{"", ""},
	}

	for _, tt := range tests {
		result := tunnelRouter.getSubdomain(tt.host)
		if result != tt.expected {
			t.Errorf("getSubdomain(%s) = %s, want %s", tt.host, result, tt.expected)
		}
	}
}

func TestTunnelRouterStats(t *testing.T) {
	server := NewServer()
	tunnelRouter := NewTunnelRouter(server)

	host1 := "test1.localhost:8080"
	host2 := "test2.localhost:8080"

	tunnelRouter.getSubdomain(host1)
	tunnelRouter.getSubdomain(host2)

	tunnelRouter.getSubdomain(host1)

	stats := tunnelRouter.GetStats()

	if stats["cache_hits"].(int64) != 1 {
		t.Errorf("Expected 1 cache hit, got %v", stats["cache_hits"])
	}

	if stats["cache_misses"].(int64) != 2 {
		t.Errorf("Expected 2 cache misses, got %v", stats["cache_misses"])
	}
}
