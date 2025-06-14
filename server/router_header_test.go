package server

import (
	"crypto/tls"
	"net/http/httptest"
	"testing"
)

func TestTunnelRouter_getClientIP(t *testing.T) {
	server := &Server{
		Tunnels: make(map[string]*Tunnel),
	}
	router := NewTunnelRouter(server)

	tests := []struct {
		name       string
		headers    map[string]string
		remoteAddr string
		expectedIP string
	}{
		{
			name: "X-Forwarded-For single IP",
			headers: map[string]string{
				"X-Forwarded-For": "192.168.1.100",
			},
			remoteAddr: "10.0.0.1:12345",
			expectedIP: "192.168.1.100",
		},
		{
			name: "X-Forwarded-For multiple IPs",
			headers: map[string]string{
				"X-Forwarded-For": "192.168.1.100, 10.0.0.50, 172.16.0.1",
			},
			remoteAddr: "10.0.0.1:12345",
			expectedIP: "192.168.1.100",
		},
		{
			name: "X-Real-IP when no X-Forwarded-For",
			headers: map[string]string{
				"X-Real-IP": "203.0.113.45",
			},
			remoteAddr: "10.0.0.1:12345",
			expectedIP: "203.0.113.45",
		},
		{
			name: "X-Forwarded header format",
			headers: map[string]string{
				"X-Forwarded": "for=198.51.100.17;proto=http",
			},
			remoteAddr: "10.0.0.1:12345",
			expectedIP: "198.51.100.17",
		},
		{
			name: "X-Forwarded header with semicolon",
			headers: map[string]string{
				"X-Forwarded": "for=198.51.100.17;host=example.com;proto=https",
			},
			remoteAddr: "10.0.0.1:12345",
			expectedIP: "198.51.100.17",
		},
		{
			name:       "fallback to RemoteAddr with port",
			headers:    map[string]string{},
			remoteAddr: "203.0.113.195:54321",
			expectedIP: "203.0.113.195",
		},
		{
			name:       "fallback to RemoteAddr IPv6 with brackets",
			headers:    map[string]string{},
			remoteAddr: "[2001:db8::1]:8080",
			expectedIP: "2001:db8::1",
		},
		{
			name:       "fallback to RemoteAddr IPv6 without port",
			headers:    map[string]string{},
			remoteAddr: "2001:db8::1",
			expectedIP: "2001:db8::1",
		},
		{
			name: "X-Forwarded-For takes precedence over X-Real-IP",
			headers: map[string]string{
				"X-Forwarded-For": "192.168.1.100",
				"X-Real-IP":       "203.0.113.45",
			},
			remoteAddr: "10.0.0.1:12345",
			expectedIP: "192.168.1.100",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest("GET", "http://example.com/test", nil)
			req.RemoteAddr = tt.remoteAddr

			for k, v := range tt.headers {
				req.Header.Set(k, v)
			}

			result := router.getClientIP(req)
			if result != tt.expectedIP {
				t.Errorf("getClientIP() = %v, want %v", result, tt.expectedIP)
			}
		})
	}
}

func TestTunnelRouter_prepareForwardingHeaders(t *testing.T) {
	server := &Server{
		Tunnels: make(map[string]*Tunnel),
	}
	router := NewTunnelRouter(server)

	tests := []struct {
		name            string
		originalHeaders map[string][]string
		host            string
		remoteAddr      string
		tls             bool
		expectedHeaders map[string]string
	}{
		{
			name: "basic forwarding headers",
			originalHeaders: map[string][]string{
				"User-Agent":   {"TestClient/1.0"},
				"Accept":       {"application/json"},
				"Content-Type": {"application/json"},
			},
			host:       "api.example.com",
			remoteAddr: "192.168.1.100:12345",
			tls:        false,
			expectedHeaders: map[string]string{
				"X-Forwarded-For":    "192.168.1.100",
				"X-Forwarded-Host":   "api.example.com",
				"X-Forwarded-Proto":  "http",
				"X-Real-IP":          "192.168.1.100",
				"X-Forwarded-Server": "api.example.com",
				"User-Agent":         "TestClient/1.0",
				"Accept":             "application/json",
				"Content-Type":       "application/json",
			},
		},
		{
			name: "HTTPS request",
			originalHeaders: map[string][]string{
				"User-Agent": {"TestClient/1.0"},
			},
			host:       "secure.example.com",
			remoteAddr: "203.0.113.45:54321",
			tls:        true,
			expectedHeaders: map[string]string{
				"X-Forwarded-For":    "203.0.113.45",
				"X-Forwarded-Host":   "secure.example.com",
				"X-Forwarded-Proto":  "https",
				"X-Real-IP":          "203.0.113.45",
				"X-Forwarded-Server": "secure.example.com",
				"User-Agent":         "TestClient/1.0",
			},
		},
		{
			name: "existing X-Forwarded-For chain",
			originalHeaders: map[string][]string{
				"X-Forwarded-For": {"192.168.1.50, 10.0.0.25"},
				"User-Agent":      {"TestClient/1.0"},
			},
			host:       "chain.example.com",
			remoteAddr: "172.16.0.1:8080",
			tls:        false,
			expectedHeaders: map[string]string{
				"X-Forwarded-For":    "192.168.1.50, 10.0.0.25, 172.16.0.1",
				"X-Forwarded-Host":   "chain.example.com",
				"X-Forwarded-Proto":  "http",
				"X-Real-IP":          "192.168.1.50",
				"X-Forwarded-Server": "chain.example.com",
				"User-Agent":         "TestClient/1.0",
			},
		},
		{
			name: "existing X-Forwarded-Proto preserved",
			originalHeaders: map[string][]string{
				"X-Forwarded-Proto": {"https"},
				"User-Agent":        {"TestClient/1.0"},
			},
			host:       "proxy.example.com",
			remoteAddr: "198.51.100.1:443",
			tls:        false,
			expectedHeaders: map[string]string{
				"X-Forwarded-For":    "198.51.100.1",
				"X-Forwarded-Host":   "proxy.example.com",
				"X-Forwarded-Proto":  "https",
				"X-Real-IP":          "198.51.100.1",
				"X-Forwarded-Server": "proxy.example.com",
				"User-Agent":         "TestClient/1.0",
			},
		},
		{
			name: "existing X-Forwarded-Server preserved",
			originalHeaders: map[string][]string{
				"X-Forwarded-Server": {"upstream.example.com"},
				"User-Agent":         {"TestClient/1.0"},
			},
			host:       "current.example.com",
			remoteAddr: "203.0.113.100:9000",
			tls:        false,
			expectedHeaders: map[string]string{
				"X-Forwarded-For":    "203.0.113.100",
				"X-Forwarded-Host":   "current.example.com",
				"X-Forwarded-Proto":  "http",
				"X-Real-IP":          "203.0.113.100",
				"X-Forwarded-Server": "upstream.example.com",
				"User-Agent":         "TestClient/1.0",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest("GET", "http://"+tt.host+"/test", nil)
			req.Host = tt.host
			req.RemoteAddr = tt.remoteAddr
			if tt.tls {
				req.TLS = &tls.ConnectionState{}
			}

			for k, values := range tt.originalHeaders {
				for _, v := range values {
					req.Header.Add(k, v)
				}
			}

			result := router.prepareForwardingHeaders(req)

			for expectedKey, expectedValue := range tt.expectedHeaders {
				if values, exists := result[expectedKey]; !exists {
					t.Errorf("Expected header %s not found", expectedKey)
				} else if len(values) == 0 || values[0] != expectedValue {
					t.Errorf("Header %s = %v, want %v", expectedKey, values, []string{expectedValue})
				}
			}

			if len(result["X-Forwarded-Host"]) > 0 && result["X-Forwarded-Host"][0] != tt.host {
				t.Errorf("X-Forwarded-Host = %v, want %v", result["X-Forwarded-Host"][0], tt.host)
			}
		})
	}
}

func TestTunnelRouter_prepareForwardingHeaders_EmptyHost(t *testing.T) {
	server := &Server{
		Tunnels: make(map[string]*Tunnel),
	}
	router := NewTunnelRouter(server)

	req := httptest.NewRequest("GET", "http://example.com/test", nil)
	req.Host = ""
	req.RemoteAddr = "192.168.1.1:12345"

	result := router.prepareForwardingHeaders(req)

	if _, exists := result["X-Forwarded-Host"]; exists {
		t.Error("X-Forwarded-Host should not be set when host is empty")
	}

	if len(result["X-Forwarded-For"]) == 0 || result["X-Forwarded-For"][0] != "192.168.1.1" {
		t.Errorf("X-Forwarded-For = %v, want %v", result["X-Forwarded-For"], []string{"192.168.1.1"})
	}
}
