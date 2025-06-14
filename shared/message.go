package shared

type Message struct {
	Type      string              `json:"type"`
	TunnelID  string              `json:"tunnel_id,omitempty"`
	RequestID string              `json:"request_id,omitempty"`
	Method    string              `json:"method,omitempty"`
	Path      string              `json:"path,omitempty"`
	Headers   map[string][]string `json:"headers,omitempty"`
	Body      []byte              `json:"body,omitempty"`
	Status    int                 `json:"status,omitempty"`
}
