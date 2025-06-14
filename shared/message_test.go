package shared

import (
	"encoding/json"
	"reflect"
	"testing"
)

// messagesEqual compares two Message structs for equality,
// treating nil and empty maps/slices as equivalent
func messagesEqual(a, b Message) bool {
	if a.Type != b.Type ||
		a.RequestID != b.RequestID ||
		a.Method != b.Method ||
		a.Path != b.Path ||
		a.Status != b.Status {
		return false
	}

	// compare headers - treat nil and empty maps as equal
	if !headersEqual(a.Headers, b.Headers) {
		return false
	}

	// compare body - treat nil and empty slices as equal
	return bodiesEqual(a.Body, b.Body)
}

// headersEqual compares two header maps, treating nil and empty maps as equal
func headersEqual(a, b map[string][]string) bool {
	if len(a) == 0 && len(b) == 0 {
		return true
	}
	return reflect.DeepEqual(a, b)
}

// bodiesEqual compares two byte slices, treating nil and empty slices as equal
func bodiesEqual(a, b []byte) bool {
	if len(a) == 0 && len(b) == 0 {
		return true
	}
	return reflect.DeepEqual(a, b)
}

func TestMessageJSONSerialization(t *testing.T) {
	tests := []struct {
		name    string
		message Message
	}{
		{
			name: "complete message",
			message: Message{
				Type:      "request",
				RequestID: "test-123",
				Method:    "GET",
				Path:      "/api/test",
				Headers: map[string][]string{
					"Content-Type":  {"application/json"},
					"Authorization": {"Bearer token123"},
				},
				Body:   []byte("test body content"),
				Status: 200,
			},
		},
		{
			name: "minimal message",
			message: Message{
				Type: "ping",
			},
		},
		{
			name: "response message",
			message: Message{
				Type:      "response",
				RequestID: "req-456",
				Status:    404,
				Headers: map[string][]string{
					"Content-Type": {"text/plain"},
				},
				Body: []byte("Not Found"),
			},
		},
		{
			name: "empty headers",
			message: Message{
				Type:    "request",
				Method:  "POST",
				Path:    "/submit",
				Headers: map[string][]string{},
				Body:    []byte("{}"),
			},
		},
		{
			name: "nil headers",
			message: Message{
				Type:   "request",
				Method: "DELETE",
				Path:   "/resource/123",
			},
		},
		{
			name: "empty body",
			message: Message{
				Type:   "request",
				Method: "HEAD",
				Path:   "/health",
				Body:   []byte{},
			},
		},
		{
			name: "nil body",
			message: Message{
				Type:   "request",
				Method: "OPTIONS",
				Path:   "/api",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name+" marshal", func(t *testing.T) {
			data, err := json.Marshal(tt.message)
			if err != nil {
				t.Errorf("failed to marshal message: %v", err)
				return
			}

			if len(data) == 0 {
				t.Error("marshaled data is empty")
			}
		})

		t.Run(tt.name+" roundtrip", func(t *testing.T) {
			// marshal
			data, err := json.Marshal(tt.message)
			if err != nil {
				t.Errorf("failed to marshal message: %v", err)
				return
			}

			// unmarshal
			var unmarshaled Message
			err = json.Unmarshal(data, &unmarshaled)
			if err != nil {
				t.Errorf("failed to unmarshal message: %v", err)
				return
			}

			// compare
			if !messagesEqual(tt.message, unmarshaled) {
				t.Errorf("roundtrip failed:\noriginal:  %+v\nunmarshaled: %+v", tt.message, unmarshaled)
			}
		})
	}
}

func TestMessageFieldValidation(t *testing.T) {
	tests := []struct {
		name     string
		jsonData string
		wantErr  bool
	}{
		{
			name:     "valid JSON",
			jsonData: `{"type":"request","method":"GET","path":"/test"}`,
			wantErr:  false,
		},
		{
			name:     "invalid JSON",
			jsonData: `{"type":"request","method":"GET"`,
			wantErr:  true,
		},
		{
			name:     "empty JSON object",
			jsonData: `{}`,
			wantErr:  false,
		},
		{
			name:     "null values",
			jsonData: `{"type":null,"method":null}`,
			wantErr:  false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var msg Message
			err := json.Unmarshal([]byte(tt.jsonData), &msg)

			if tt.wantErr && err == nil {
				t.Error("expected error but got none")
			}
			if !tt.wantErr && err != nil {
				t.Errorf("unexpected error: %v", err)
			}
		})
	}
}

func TestMessageHeaders(t *testing.T) {
	tests := []struct {
		name    string
		headers map[string][]string
	}{
		{
			name: "single value headers",
			headers: map[string][]string{
				"Content-Type": {"application/json"},
				"Accept":       {"application/json"},
			},
		},
		{
			name: "multi-value headers",
			headers: map[string][]string{
				"Accept":        {"text/html", "application/xml"},
				"Cache-Control": {"no-cache", "no-store"},
			},
		},
		{
			name: "empty value headers",
			headers: map[string][]string{
				"X-Custom": {""},
				"X-Empty":  {},
			},
		},
		{
			name: "mixed case headers",
			headers: map[string][]string{
				"Content-TYPE": {"text/plain"},
				"accept":       {"*/*"},
				"X-Custom":     {"value"},
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			msg := Message{
				Type:    "request",
				Headers: tt.headers,
			}

			data, err := json.Marshal(msg)
			if err != nil {
				t.Errorf("failed to marshal: %v", err)
				return
			}

			var unmarshaled Message
			err = json.Unmarshal(data, &unmarshaled)
			if err != nil {
				t.Errorf("failed to unmarshal: %v", err)
				return
			}

			if !messagesEqual(msg, unmarshaled) {
				t.Errorf("headers mismatch:\noriginal:    %+v\nunmarshaled: %+v", msg.Headers, unmarshaled.Headers)
			}
		})
	}
}

func TestMessageBodyHandling(t *testing.T) {
	tests := []struct {
		name string
		body []byte
	}{
		{
			name: "text body",
			body: []byte("hello world"),
		},
		{
			name: "json body",
			body: []byte(`{"key":"value","number":42}`),
		},
		{
			name: "binary body",
			body: []byte{0x00, 0x01, 0x02, 0xFF, 0xFE},
		},
		{
			name: "empty body",
			body: []byte{},
		},
		{
			name: "nil body",
			body: nil,
		},
		{
			name: "large body",
			body: make([]byte, 10000), // 10KB of zeros
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			msg := Message{
				Type: "request",
				Body: tt.body,
			}

			data, err := json.Marshal(msg)
			if err != nil {
				t.Errorf("failed to marshal: %v", err)
				return
			}

			var unmarshaled Message
			err = json.Unmarshal(data, &unmarshaled)
			if err != nil {
				t.Errorf("failed to unmarshal: %v", err)
				return
			}

			if !messagesEqual(msg, unmarshaled) {
				t.Errorf("body mismatch:\noriginal:    %+v\nunmarshaled: %+v", msg.Body, unmarshaled.Body)
			}
		})
	}
}

// benchmark tests
func BenchmarkMessageMarshal(b *testing.B) {
	msg := Message{
		Type:      "request",
		RequestID: "bench-test-123",
		Method:    "POST",
		Path:      "/api/benchmark",
		Headers: map[string][]string{
			"Content-Type":  {"application/json"},
			"Authorization": {"Bearer token123"},
			"User-Agent":    {"benchmark-client/1.0"},
		},
		Body: []byte(`{"test":"data","number":42,"array":[1,2,3]}`),
	}

	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		_, err := json.Marshal(msg)
		if err != nil {
			b.Fatal(err)
		}
	}
}

func BenchmarkMessageUnmarshal(b *testing.B) {
	data := []byte(`{"type":"request","request_id":"bench-test-123","method":"POST","path":"/api/benchmark","headers":{"Content-Type":["application/json"],"Authorization":["Bearer token123"]},"body":"eyJ0ZXN0IjoiZGF0YSJ9"}`)

	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		var msg Message
		err := json.Unmarshal(data, &msg)
		if err != nil {
			b.Fatal(err)
		}
	}
}

func BenchmarkMessageRoundtrip(b *testing.B) {
	msg := Message{
		Type:      "request",
		RequestID: "bench-test-123",
		Method:    "GET",
		Path:      "/api/test",
		Headers: map[string][]string{
			"Accept": {"application/json"},
		},
		Body: []byte("test data"),
	}

	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		data, err := json.Marshal(msg)
		if err != nil {
			b.Fatal(err)
		}

		var unmarshaled Message
		err = json.Unmarshal(data, &unmarshaled)
		if err != nil {
			b.Fatal(err)
		}
	}
}
