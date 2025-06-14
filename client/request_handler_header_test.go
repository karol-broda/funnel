package client

import (
	"net/http/httptest"
	"testing"
)

func TestClient_shouldSkipHeader(t *testing.T) {
	client := &Client{
		TunnelID:  "test-tunnel",
		LocalAddr: "localhost:3000",
	}

	tests := []struct {
		name       string
		headerName string
		shouldSkip bool
	}{
		{
			name:       "skip Connection header",
			headerName: "Connection",
			shouldSkip: true,
		},
		{
			name:       "skip connection header lowercase",
			headerName: "connection",
			shouldSkip: true,
		},
		{
			name:       "skip Upgrade header",
			headerName: "Upgrade",
			shouldSkip: true,
		},
		{
			name:       "skip upgrade header lowercase",
			headerName: "upgrade",
			shouldSkip: true,
		},
		{
			name:       "skip Proxy-Connection header",
			headerName: "Proxy-Connection",
			shouldSkip: true,
		},
		{
			name:       "skip Proxy-Authenticate header",
			headerName: "Proxy-Authenticate",
			shouldSkip: true,
		},
		{
			name:       "skip Proxy-Authorization header",
			headerName: "Proxy-Authorization",
			shouldSkip: true,
		},
		{
			name:       "skip TE header",
			headerName: "TE",
			shouldSkip: true,
		},
		{
			name:       "skip Trailer header",
			headerName: "Trailer",
			shouldSkip: true,
		},
		{
			name:       "skip Transfer-Encoding header",
			headerName: "Transfer-Encoding",
			shouldSkip: true,
		},
		{
			name:       "forward User-Agent header",
			headerName: "User-Agent",
			shouldSkip: false,
		},
		{
			name:       "forward X-Forwarded-For header",
			headerName: "X-Forwarded-For",
			shouldSkip: false,
		},
		{
			name:       "forward X-Forwarded-Host header",
			headerName: "X-Forwarded-Host",
			shouldSkip: false,
		},
		{
			name:       "forward X-Forwarded-Proto header",
			headerName: "X-Forwarded-Proto",
			shouldSkip: false,
		},
		{
			name:       "forward X-Real-IP header",
			headerName: "X-Real-IP",
			shouldSkip: false,
		},
		{
			name:       "forward Content-Type header",
			headerName: "Content-Type",
			shouldSkip: false,
		},
		{
			name:       "forward Accept header",
			headerName: "Accept",
			shouldSkip: false,
		},
		{
			name:       "forward Authorization header",
			headerName: "Authorization",
			shouldSkip: false,
		},
		{
			name:       "forward custom header",
			headerName: "X-Custom-Header",
			shouldSkip: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := client.shouldSkipHeader(tt.headerName)
			if result != tt.shouldSkip {
				t.Errorf("shouldSkipHeader(%s) = %v, want %v", tt.headerName, result, tt.shouldSkip)
			}
		})
	}
}

func TestClient_setRequestHeaders(t *testing.T) {
	tests := []struct {
		name              string
		inputHeaders      map[string][]string
		localAddr         string
		expectedSkipped   []string
		expectedForwarded []string
		expectedHost      string
	}{
		{
			name: "forward proxy headers",
			inputHeaders: map[string][]string{
				"X-Forwarded-For":   {"192.168.1.100"},
				"X-Forwarded-Host":  {"api.example.com"},
				"X-Forwarded-Proto": {"https"},
				"X-Real-IP":         {"203.0.113.45"},
				"User-Agent":        {"TestClient/1.0"},
				"Accept":            {"application/json"},
			},
			localAddr:         "localhost:8080",
			expectedSkipped:   []string{},
			expectedForwarded: []string{"X-Forwarded-For", "X-Forwarded-Host", "X-Forwarded-Proto", "X-Real-IP", "User-Agent", "Accept"},
			expectedHost:      "localhost:8080",
		},
		{
			name: "skip connection headers",
			inputHeaders: map[string][]string{
				"Connection":        {"keep-alive"},
				"Upgrade":           {"websocket"},
				"Transfer-Encoding": {"chunked"},
				"TE":                {"trailers"},
				"User-Agent":        {"TestClient/1.0"},
				"Content-Type":      {"application/json"},
			},
			localAddr:         "localhost:3000",
			expectedSkipped:   []string{"Connection", "Upgrade", "Transfer-Encoding", "TE"},
			expectedForwarded: []string{"User-Agent", "Content-Type"},
			expectedHost:      "localhost:3000",
		},
		{
			name: "handle Host header specially",
			inputHeaders: map[string][]string{
				"Host":         {"tunnel.example.com"},
				"User-Agent":   {"TestClient/1.0"},
				"Content-Type": {"application/json"},
			},
			localAddr:         "localhost:9000",
			expectedSkipped:   []string{},
			expectedForwarded: []string{"User-Agent", "Content-Type"},
			expectedHost:      "localhost:9000",
		},
		{
			name: "mixed headers with proxy and connection headers",
			inputHeaders: map[string][]string{
				"X-Forwarded-For":   {"192.168.1.100, 10.0.0.1"},
				"X-Forwarded-Host":  {"api.example.com"},
				"Connection":        {"close"},
				"Proxy-Connection":  {"keep-alive"},
				"User-Agent":        {"TestClient/1.0"},
				"Authorization":     {"Bearer token123"},
				"Content-Type":      {"application/json"},
				"X-Custom-Header":   {"custom-value"},
				"Transfer-Encoding": {"gzip"},
			},
			localAddr:         "localhost:5000",
			expectedSkipped:   []string{"Connection", "Proxy-Connection", "Transfer-Encoding"},
			expectedForwarded: []string{"X-Forwarded-For", "X-Forwarded-Host", "User-Agent", "Authorization", "Content-Type", "X-Custom-Header"},
			expectedHost:      "localhost:5000",
		},
		{
			name: "no Host header provided",
			inputHeaders: map[string][]string{
				"User-Agent":   {"TestClient/1.0"},
				"Content-Type": {"application/json"},
			},
			localAddr:         "localhost:4000",
			expectedSkipped:   []string{},
			expectedForwarded: []string{"User-Agent", "Content-Type"},
			expectedHost:      "localhost:4000",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			client := &Client{
				TunnelID:  "test-headers",
				LocalAddr: tt.localAddr,
			}

			req := httptest.NewRequest("GET", "http://"+tt.localAddr+"/test", nil)

			client.setRequestHeaders(req, tt.inputHeaders)

			for _, skippedHeader := range tt.expectedSkipped {
				if values := req.Header.Values(skippedHeader); len(values) > 0 {
					t.Errorf("Header %s should be skipped but found values: %v", skippedHeader, values)
				}
			}

			for _, forwardedHeader := range tt.expectedForwarded {
				if values := req.Header.Values(forwardedHeader); len(values) == 0 {
					t.Errorf("Header %s should be forwarded but not found", forwardedHeader)
				}
			}

			if req.Host != tt.expectedHost {
				t.Errorf("Host header = %s, want %s", req.Host, tt.expectedHost)
			}

			if tt.inputHeaders["X-Forwarded-For"] != nil {
				if forwardedFor := req.Header.Get("X-Forwarded-For"); forwardedFor != tt.inputHeaders["X-Forwarded-For"][0] {
					t.Errorf("X-Forwarded-For = %s, want %s", forwardedFor, tt.inputHeaders["X-Forwarded-For"][0])
				}
			}
		})
	}
}

func TestClient_setRequestHeaders_HeaderValues(t *testing.T) {
	client := &Client{
		TunnelID:  "test-values",
		LocalAddr: "localhost:3000",
	}

	tests := []struct {
		name          string
		inputHeaders  map[string][]string
		headerToTest  string
		expectedValue string
	}{
		{
			name: "single value header",
			inputHeaders: map[string][]string{
				"Authorization": {"Bearer token123"},
			},
			headerToTest:  "Authorization",
			expectedValue: "Bearer token123",
		},
		{
			name: "multiple value header - first value",
			inputHeaders: map[string][]string{
				"Accept": {"application/json", "text/html"},
			},
			headerToTest:  "Accept",
			expectedValue: "application/json",
		},
		{
			name: "forwarded header preservation",
			inputHeaders: map[string][]string{
				"X-Forwarded-Proto": {"https"},
			},
			headerToTest:  "X-Forwarded-Proto",
			expectedValue: "https",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest("GET", "http://localhost:3000/test", nil)

			client.setRequestHeaders(req, tt.inputHeaders)

			actualValue := req.Header.Get(tt.headerToTest)
			if actualValue != tt.expectedValue {
				t.Errorf("Header %s = %s, want %s", tt.headerToTest, actualValue, tt.expectedValue)
			}
		})
	}
}
