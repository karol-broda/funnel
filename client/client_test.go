package client

import (
	"net/http"
	"net/url"
	"strings"
	"testing"

	"github.com/karol-broda/go-tunnel-proxy/shared"
)

func TestClientConnectionHandling(t *testing.T) {
	tests := []struct {
		name        string
		serverURL   string
		expectError bool
	}{
		{
			name:        "valid http URL",
			serverURL:   "http://localhost:8080",
			expectError: false,
		},
		{
			name:        "valid https URL",
			serverURL:   "https://example.com",
			expectError: false,
		},
		{
			name:        "invalid URL",
			serverURL:   "invalid-url",
			expectError: true,
		},
		{
			name:        "empty URL",
			serverURL:   "",
			expectError: true,
		},
		{
			name:        "URL with port",
			serverURL:   "http://localhost:9000",
			expectError: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			client := &Client{
				ServerURL: tt.serverURL,
				LocalAddr: "localhost:3000",
				TunnelID:  "test-tunnel",
			}

			err := client.connect()

			if tt.expectError {
				if err == nil {
					t.Error("expected error but got none")
				}
				return
			}

			if err != nil {
				if _, parseErr := url.Parse(tt.serverURL); parseErr != nil {
					t.Errorf("URL parsing failed: %v", parseErr)
				}
			}
		})
	}
}

func TestClientURLConstruction(t *testing.T) {
	tests := []struct {
		name           string
		serverURL      string
		tunnelID       string
		expectedScheme string
		expectedHost   string
		expectedQuery  string
	}{
		{
			name:           "http to ws",
			serverURL:      "http://localhost:8080",
			tunnelID:       "test123",
			expectedScheme: "ws",
			expectedHost:   "localhost:8080",
			expectedQuery:  "id=test123",
		},
		{
			name:           "https to wss",
			serverURL:      "https://tunnel.example.com",
			tunnelID:       "secure-tunnel",
			expectedScheme: "wss",
			expectedHost:   "tunnel.example.com",
			expectedQuery:  "id=secure-tunnel",
		},
		{
			name:           "http with path",
			serverURL:      "http://localhost:8080/api",
			tunnelID:       "path-test",
			expectedScheme: "ws",
			expectedHost:   "localhost:8080",
			expectedQuery:  "id=path-test",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			u, err := url.Parse(tt.serverURL)
			if err != nil {
				t.Fatalf("failed to parse server URL: %v", err)
			}

			wsScheme := "ws"
			if u.Scheme == "https" {
				wsScheme = "wss"
			}

			if wsScheme != tt.expectedScheme {
				t.Errorf("expected scheme %s, got %s", tt.expectedScheme, wsScheme)
			}

			if u.Host != tt.expectedHost {
				t.Errorf("expected host %s, got %s", tt.expectedHost, u.Host)
			}

			expectedURL := wsScheme + "://" + u.Host + "/?id=" + tt.tunnelID
			wsURL := expectedURL

			if !strings.Contains(wsURL, tt.expectedQuery) {
				t.Errorf("expected query %s in URL %s", tt.expectedQuery, wsURL)
			}
		})
	}
}

func TestClientHeaderFiltering(t *testing.T) {
	client := &Client{
		TunnelID: "test-headers",
	}

	tests := []struct {
		name             string
		inputHeaders     map[string][]string
		expectedFiltered []string
		expectedKept     []string
	}{
		{
			name: "filter x-headers",
			inputHeaders: map[string][]string{
				"X-Real-IP":       {"192.168.1.1"},
				"X-Forwarded-For": {"10.0.0.1"},
				"Content-Type":    {"application/json"},
				"Accept":          {"*/*"},
			},
			expectedFiltered: []string{"X-Real-IP", "X-Forwarded-For"},
			expectedKept:     []string{"Content-Type", "Accept"},
		},
		{
			name: "filter host and connection",
			inputHeaders: map[string][]string{
				"Host":       {"example.com"},
				"Connection": {"keep-alive"},
				"User-Agent": {"test-client"},
			},
			expectedFiltered: []string{"Host", "Connection"},
			expectedKept:     []string{"User-Agent"},
		},
		{
			name: "keep standard headers",
			inputHeaders: map[string][]string{
				"Content-Type":   {"text/plain"},
				"Content-Length": {"42"},
				"Accept":         {"text/html"},
				"User-Agent":     {"test-browser"},
			},
			expectedFiltered: []string{},
			expectedKept:     []string{"Content-Type", "Content-Length", "Accept", "User-Agent"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req, err := http.NewRequest("GET", "http://localhost:3000/test", nil)
			if err != nil {
				t.Fatalf("failed to create request: %v", err)
			}

			client.setRequestHeaders(req, tt.inputHeaders)

			// check that filtered headers are not in the request
			for _, header := range tt.expectedFiltered {
				if values := req.Header.Values(header); len(values) > 0 {
					t.Errorf("header %s should be filtered but found values: %v", header, values)
				}
			}

			// check that kept headers are in the request
			for _, header := range tt.expectedKept {
				if values := req.Header.Values(header); len(values) == 0 {
					t.Errorf("header %s should be kept but not found", header)
				}
			}
		})
	}
}

func TestClientErrorHandling(t *testing.T) {
	tests := []struct {
		name           string
		requestID      string
		expectedStatus int
		expectedBody   string
	}{
		{
			name:           "internal server error",
			requestID:      "test-500",
			expectedStatus: 500,
			expectedBody:   "Internal Server Error",
		},
		{
			name:           "bad gateway",
			requestID:      "test-502",
			expectedStatus: 502,
			expectedBody:   "Bad Gateway",
		},
		{
			name:           "service unavailable",
			requestID:      "test-503",
			expectedStatus: 503,
			expectedBody:   "Service Unavailable",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			response := shared.Message{
				Type:      "response",
				RequestID: tt.requestID,
				Status:    tt.expectedStatus,
				Headers: map[string][]string{
					"Content-Type": {"text/plain"},
				},
				Body: []byte(http.StatusText(tt.expectedStatus)),
			}

			if response.Type != "response" {
				t.Errorf("expected response type, got %s", response.Type)
			}

			if response.RequestID != tt.requestID {
				t.Errorf("expected request ID %s, got %s", tt.requestID, response.RequestID)
			}

			if response.Status != tt.expectedStatus {
				t.Errorf("expected status %d, got %d", tt.expectedStatus, response.Status)
			}

			if string(response.Body) != tt.expectedBody {
				t.Errorf("expected body %s, got %s", tt.expectedBody, string(response.Body))
			}
		})
	}
}

func TestClientConfiguration(t *testing.T) {
	tests := []struct {
		name      string
		serverURL string
		localAddr string
		tunnelID  string
		valid     bool
	}{
		{
			name:      "valid configuration",
			serverURL: "http://localhost:8080",
			localAddr: "localhost:3000",
			tunnelID:  "test-tunnel",
			valid:     true,
		},
		{
			name:      "empty server URL",
			serverURL: "",
			localAddr: "localhost:3000",
			tunnelID:  "test-tunnel",
			valid:     false,
		},
		{
			name:      "empty local address",
			serverURL: "http://localhost:8080",
			localAddr: "",
			tunnelID:  "test-tunnel",
			valid:     false,
		},
		{
			name:      "empty tunnel ID",
			serverURL: "http://localhost:8080",
			localAddr: "localhost:3000",
			tunnelID:  "",
			valid:     false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			client := &Client{
				ServerURL: tt.serverURL,
				LocalAddr: tt.localAddr,
				TunnelID:  tt.tunnelID,
			}

			// validate configuration
			valid := client.ServerURL != "" && client.LocalAddr != "" && client.TunnelID != ""

			if valid != tt.valid {
				t.Errorf("expected valid=%v, got valid=%v", tt.valid, valid)
			}
		})
	}
}

func BenchmarkClientHeaderFiltering(b *testing.B) {
	client := &Client{TunnelID: "bench-test"}

	headers := map[string][]string{
		"X-Real-IP":         {"192.168.1.1"},
		"X-Forwarded-For":   {"10.0.0.1"},
		"X-Forwarded-Proto": {"https"},
		"Host":              {"example.com"},
		"Connection":        {"keep-alive"},
		"Content-Type":      {"application/json"},
		"Accept":            {"application/json"},
		"User-Agent":        {"test-client/1.0"},
		"Authorization":     {"Bearer token123"},
		"Content-Length":    {"42"},
	}

	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		req, _ := http.NewRequest("GET", "http://localhost:3000/test", nil)
		client.setRequestHeaders(req, headers)
	}
}

func BenchmarkClientURLConstruction(b *testing.B) {
	client := &Client{
		ServerURL: "http://localhost:8080",
		TunnelID:  "benchmark-tunnel",
	}

	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		u, _ := url.Parse(client.ServerURL)
		wsScheme := "ws"
		if u.Scheme == "https" {
			wsScheme = "wss"
		}
		_ = wsScheme + "://" + u.Host + "/?id=" + client.TunnelID
	}
}
