package server

import (
	"crypto/rand"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/karol-broda/funnel/shared"
)

type TokenRecord struct {
	Name      string    `json:"name"`
	Hash      string    `json:"hash"`
	Prefix    string    `json:"prefix"`
	CreatedAt time.Time `json:"created_at"`
	Revoked   bool      `json:"revoked,omitempty"`
}

type TokenStore struct {
	path    string
	records []TokenRecord
	mu      sync.RWMutex
	enabled bool
}

func NewTokenStore(path string) (*TokenStore, error) {
	logger := shared.GetLogger("server.auth")

	ts := &TokenStore{
		path:    path,
		records: []TokenRecord{},
		enabled: true,
	}

	if path == "" {
		ts.enabled = false
		logger.Info().Msg("token store disabled - no path configured")
		return ts, nil
	}

	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0700); err != nil {
		return nil, fmt.Errorf("failed to create token store directory: %w", err)
	}

	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			logger.Info().Str("path", path).Msg("token store file not found, starting with empty store")
			return ts, nil
		}
		return nil, fmt.Errorf("failed to read token store: %w", err)
	}

	if err := json.Unmarshal(data, &ts.records); err != nil {
		return nil, fmt.Errorf("failed to parse token store: %w", err)
	}

	activeCount := 0
	for _, r := range ts.records {
		if !r.Revoked {
			activeCount++
		}
	}

	logger.Info().
		Str("path", path).
		Int("total_tokens", len(ts.records)).
		Int("active_tokens", activeCount).
		Msg("token store loaded")

	return ts, nil
}

func (ts *TokenStore) IsEnabled() bool {
	return ts.enabled
}

func (ts *TokenStore) save() error {
	if ts.path == "" {
		return nil
	}

	data, err := json.MarshalIndent(ts.records, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal tokens: %w", err)
	}

	tmpPath := ts.path + ".tmp"
	if err := os.WriteFile(tmpPath, data, 0600); err != nil {
		return fmt.Errorf("failed to write token store: %w", err)
	}

	if err := os.Rename(tmpPath, ts.path); err != nil {
		os.Remove(tmpPath)
		return fmt.Errorf("failed to rename token store: %w", err)
	}

	return nil
}

func (ts *TokenStore) Create(name string) (string, error) {
	logger := shared.GetLogger("server.auth")

	ts.mu.Lock()
	defer ts.mu.Unlock()

	for _, r := range ts.records {
		if r.Name == name && !r.Revoked {
			return "", fmt.Errorf("token with name %q already exists", name)
		}
	}

	plainToken, err := generateSecureToken()
	if err != nil {
		return "", fmt.Errorf("failed to generate token: %w", err)
	}

	record := TokenRecord{
		Name:      name,
		Hash:      hashToken(plainToken),
		Prefix:    plainToken[:10],
		CreatedAt: time.Now(),
		Revoked:   false,
	}

	ts.records = append(ts.records, record)

	if err := ts.save(); err != nil {
		ts.records = ts.records[:len(ts.records)-1]
		return "", fmt.Errorf("failed to save token: %w", err)
	}

	logger.Info().
		Str("name", name).
		Str("prefix", record.Prefix).
		Msg("token created")

	return plainToken, nil
}

func (ts *TokenStore) Validate(plainToken string) (*TokenRecord, bool) {
	if !ts.enabled {
		return nil, true
	}

	if plainToken == "" {
		return nil, false
	}

	hash := hashToken(plainToken)

	ts.mu.RLock()
	defer ts.mu.RUnlock()

	for i := range ts.records {
		if ts.records[i].Revoked {
			continue
		}
		if subtle.ConstantTimeCompare([]byte(ts.records[i].Hash), []byte(hash)) == 1 {
			return &ts.records[i], true
		}
	}

	return nil, false
}

func (ts *TokenStore) Revoke(name string) error {
	logger := shared.GetLogger("server.auth")

	ts.mu.Lock()
	defer ts.mu.Unlock()

	found := false
	for i := range ts.records {
		if ts.records[i].Name == name && !ts.records[i].Revoked {
			ts.records[i].Revoked = true
			found = true
			break
		}
	}

	if !found {
		return fmt.Errorf("token %q not found or already revoked", name)
	}

	if err := ts.save(); err != nil {
		return fmt.Errorf("failed to save token store: %w", err)
	}

	logger.Info().Str("name", name).Msg("token revoked")
	return nil
}

func (ts *TokenStore) List() []TokenRecord {
	ts.mu.RLock()
	defer ts.mu.RUnlock()

	result := make([]TokenRecord, 0)
	for _, r := range ts.records {
		if !r.Revoked {
			result = append(result, r)
		}
	}
	return result
}

func (ts *TokenStore) Count() int {
	ts.mu.RLock()
	defer ts.mu.RUnlock()

	count := 0
	for _, r := range ts.records {
		if !r.Revoked {
			count++
		}
	}
	return count
}

func generateSecureToken() (string, error) {
	bytes := make([]byte, 32)
	if _, err := rand.Read(bytes); err != nil {
		return "", err
	}
	return "sk_" + base64.RawURLEncoding.EncodeToString(bytes), nil
}

func hashToken(token string) string {
	h := sha256.Sum256([]byte(token))
	return hex.EncodeToString(h[:])
}
