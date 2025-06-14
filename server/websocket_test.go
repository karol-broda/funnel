package server

import (
	"net/http"
	"net/http/httptest"
	"net/url"
	"strings"
	"testing"

	"github.com/karol-broda/funnel/shared"
)

func TestWebSocketTunnelIDValidation(t *testing.T) {
	server := NewServer()

	tests := []struct {
		name           string
		tunnelID       string
		expectedStatus int
		expectedError  string
	}{
		{
			name:           "valid tunnel ID",
			tunnelID:       "abc123",
			expectedStatus: http.StatusInternalServerError, // websocket upgrade fails in tests due to httptest.ResponseRecorder limitations
		},
		{
			name:           "valid tunnel ID with hyphens",
			tunnelID:       "abc-123-def",
			expectedStatus: http.StatusInternalServerError, // websocket upgrade fails in tests due to httptest.ResponseRecorder limitations
		},
		{
			name:           "empty tunnel ID",
			tunnelID:       "",
			expectedStatus: http.StatusBadRequest,
			expectedError:  "tunnel id required",
		},
		{
			name:           "tunnel ID with uppercase",
			tunnelID:       "Abc123",
			expectedStatus: http.StatusBadRequest,
			expectedError:  "invalid tunnel id format",
		},
		{
			name:           "tunnel ID too short",
			tunnelID:       "ab",
			expectedStatus: http.StatusBadRequest,
			expectedError:  "invalid tunnel id format",
		},
		{
			name:           "tunnel ID too long",
			tunnelID:       strings.Repeat("a", 64),
			expectedStatus: http.StatusBadRequest,
			expectedError:  "invalid tunnel id format",
		},
		{
			name:           "tunnel ID starts with hyphen",
			tunnelID:       "-abc123",
			expectedStatus: http.StatusBadRequest,
			expectedError:  "invalid tunnel id format",
		},
		{
			name:           "tunnel ID ends with hyphen",
			tunnelID:       "abc123-",
			expectedStatus: http.StatusBadRequest,
			expectedError:  "invalid tunnel id format",
		},
		{
			name:           "tunnel ID with invalid characters",
			tunnelID:       "abc_123",
			expectedStatus: http.StatusBadRequest,
			expectedError:  "invalid tunnel id format",
		},
		{
			name:           "tunnel ID with spaces",
			tunnelID:       "abc 123",
			expectedStatus: http.StatusBadRequest,
			expectedError:  "invalid tunnel id format",
		},
		{
			name:           "tunnel ID with special characters",
			tunnelID:       "abc@123",
			expectedStatus: http.StatusBadRequest,
			expectedError:  "invalid tunnel id format",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// create request with tunnel ID
			req := httptest.NewRequest("GET", "/", nil)
			req.Header.Set("Connection", "upgrade")
			req.Header.Set("Upgrade", "websocket")
			req.Header.Set("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
			req.Header.Set("Sec-WebSocket-Version", "13")

			// add tunnel ID as query parameter
			if tt.tunnelID != "" {
				q := req.URL.Query()
				q.Add("id", tt.tunnelID)
				req.URL.RawQuery = q.Encode()
			}

			// create response recorder
			w := httptest.NewRecorder()

			// call the handler
			server.HandleWebSocket(w, req)

			// check status code
			if w.Code != tt.expectedStatus {
				t.Errorf("HandleWebSocket() status = %v, want %v", w.Code, tt.expectedStatus)
			}

			// check error message if expected
			if tt.expectedError != "" {
				body := w.Body.String()
				if !strings.Contains(body, tt.expectedError) {
					t.Errorf("HandleWebSocket() body = %v, want to contain %v", body, tt.expectedError)
				}
			}
		})
	}
}

func TestWebSocketTunnelIDConflict(t *testing.T) {
	server := NewServer()

	// add a tunnel first
	existingTunnelID := "existing123"
	server.AddTunnel(existingTunnelID, nil)
	defer server.RemoveTunnel(existingTunnelID)

	// try to create another tunnel with the same ID
	req := httptest.NewRequest("GET", "/", nil)
	req.Header.Set("Connection", "upgrade")
	req.Header.Set("Upgrade", "websocket")
	req.Header.Set("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
	req.Header.Set("Sec-WebSocket-Version", "13")

	q := req.URL.Query()
	q.Add("id", existingTunnelID)
	req.URL.RawQuery = q.Encode()

	w := httptest.NewRecorder()
	server.HandleWebSocket(w, req)

	if w.Code != http.StatusConflict {
		t.Errorf("HandleWebSocket() status = %v, want %v", w.Code, http.StatusConflict)
	}

	body := w.Body.String()
	if !strings.Contains(body, "tunnel id already in use") {
		t.Errorf("HandleWebSocket() body = %v, want to contain 'tunnel id already in use'", body)
	}
}

func TestWebSocketTunnelIDGeneration(t *testing.T) {
	// test that generated IDs are valid
	for i := 0; i < 100; i++ {
		id := shared.MustGenerateDomainSafeID()

		if err := shared.ValidateTunnelID(id); err != nil {
			t.Errorf("Generated tunnel ID %s is invalid: %v", id, err)
		}

		// ensure it's domain-safe (no uppercase)
		for _, char := range id {
			if char >= 'A' && char <= 'Z' {
				t.Errorf("Generated tunnel ID %s contains uppercase character: %c", id, char)
			}
		}
	}
}

func TestWebSocketURLParsing(t *testing.T) {
	tests := []struct {
		name     string
		rawURL   string
		expected string
	}{
		{
			name:     "simple tunnel ID",
			rawURL:   "/?id=abc123",
			expected: "abc123",
		},
		{
			name:     "tunnel ID with hyphens",
			rawURL:   "/?id=abc-123-def",
			expected: "abc-123-def",
		},
		{
			name:     "tunnel ID with other params",
			rawURL:   "/?id=abc123&other=value",
			expected: "abc123",
		},
		{
			name:     "URL encoded tunnel ID",
			rawURL:   "/?id=" + url.QueryEscape("abc-123"),
			expected: "abc-123",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest("GET", tt.rawURL, nil)
			tunnelID := req.URL.Query().Get("id")

			if tunnelID != tt.expected {
				t.Errorf("URL parsing tunnelID = %v, want %v", tunnelID, tt.expected)
			}
		})
	}
}
