package server

import (
	"os"
	"path/filepath"
	"testing"
)

func TestTokenStore_CreateAndValidate(t *testing.T) {
	tmpDir := t.TempDir()
	storePath := filepath.Join(tmpDir, "tokens.json")

	store, err := NewTokenStore(storePath)
	if err != nil {
		t.Fatalf("failed to create token store: %v", err)
	}

	plainToken, err := store.Create("test-token")
	if err != nil {
		t.Fatalf("failed to create token: %v", err)
	}

	if plainToken == "" {
		t.Fatal("expected non-empty token")
	}

	if len(plainToken) < 10 {
		t.Fatalf("token too short: %s", plainToken)
	}

	if plainToken[:3] != "sk_" {
		t.Fatalf("token should start with 'sk_', got: %s", plainToken[:3])
	}

	record, valid := store.Validate(plainToken)
	if !valid {
		t.Fatal("expected token to be valid")
	}

	if record == nil {
		t.Fatal("expected record to be non-nil")
	}

	if record.Name != "test-token" {
		t.Fatalf("expected name 'test-token', got: %s", record.Name)
	}
}

func TestTokenStore_InvalidToken(t *testing.T) {
	tmpDir := t.TempDir()
	storePath := filepath.Join(tmpDir, "tokens.json")

	store, err := NewTokenStore(storePath)
	if err != nil {
		t.Fatalf("failed to create token store: %v", err)
	}

	_, err = store.Create("test-token")
	if err != nil {
		t.Fatalf("failed to create token: %v", err)
	}

	record, valid := store.Validate("sk_invalid_token_that_does_not_exist")
	if valid {
		t.Fatal("expected invalid token to fail validation")
	}

	if record != nil {
		t.Fatal("expected record to be nil for invalid token")
	}

	record, valid = store.Validate("")
	if valid {
		t.Fatal("expected empty token to fail validation")
	}
}

func TestTokenStore_Revoke(t *testing.T) {
	tmpDir := t.TempDir()
	storePath := filepath.Join(tmpDir, "tokens.json")

	store, err := NewTokenStore(storePath)
	if err != nil {
		t.Fatalf("failed to create token store: %v", err)
	}

	plainToken, err := store.Create("test-token")
	if err != nil {
		t.Fatalf("failed to create token: %v", err)
	}

	_, valid := store.Validate(plainToken)
	if !valid {
		t.Fatal("expected token to be valid before revoke")
	}

	err = store.Revoke("test-token")
	if err != nil {
		t.Fatalf("failed to revoke token: %v", err)
	}

	_, valid = store.Validate(plainToken)
	if valid {
		t.Fatal("expected token to be invalid after revoke")
	}
}

func TestTokenStore_DuplicateName(t *testing.T) {
	tmpDir := t.TempDir()
	storePath := filepath.Join(tmpDir, "tokens.json")

	store, err := NewTokenStore(storePath)
	if err != nil {
		t.Fatalf("failed to create token store: %v", err)
	}

	_, err = store.Create("test-token")
	if err != nil {
		t.Fatalf("failed to create first token: %v", err)
	}

	_, err = store.Create("test-token")
	if err == nil {
		t.Fatal("expected error when creating duplicate token name")
	}
}

func TestTokenStore_Persistence(t *testing.T) {
	tmpDir := t.TempDir()
	storePath := filepath.Join(tmpDir, "tokens.json")

	store1, err := NewTokenStore(storePath)
	if err != nil {
		t.Fatalf("failed to create token store: %v", err)
	}

	plainToken, err := store1.Create("persistent-token")
	if err != nil {
		t.Fatalf("failed to create token: %v", err)
	}

	store2, err := NewTokenStore(storePath)
	if err != nil {
		t.Fatalf("failed to reload token store: %v", err)
	}

	record, valid := store2.Validate(plainToken)
	if !valid {
		t.Fatal("expected token to be valid after reload")
	}

	if record.Name != "persistent-token" {
		t.Fatalf("expected name 'persistent-token', got: %s", record.Name)
	}
}

func TestTokenStore_DisabledWithEmptyPath(t *testing.T) {
	store, err := NewTokenStore("")
	if err != nil {
		t.Fatalf("failed to create disabled token store: %v", err)
	}

	if store.IsEnabled() {
		t.Fatal("expected store to be disabled with empty path")
	}

	_, valid := store.Validate("any-token")
	if !valid {
		t.Fatal("expected validation to pass when store is disabled")
	}

	_, valid = store.Validate("")
	if !valid {
		t.Fatal("expected validation to pass for empty token when store is disabled")
	}
}

func TestTokenStore_List(t *testing.T) {
	tmpDir := t.TempDir()
	storePath := filepath.Join(tmpDir, "tokens.json")

	store, err := NewTokenStore(storePath)
	if err != nil {
		t.Fatalf("failed to create token store: %v", err)
	}

	tokens := store.List()
	if len(tokens) != 0 {
		t.Fatalf("expected 0 tokens, got: %d", len(tokens))
	}

	_, err = store.Create("token-1")
	if err != nil {
		t.Fatalf("failed to create token-1: %v", err)
	}

	_, err = store.Create("token-2")
	if err != nil {
		t.Fatalf("failed to create token-2: %v", err)
	}

	tokens = store.List()
	if len(tokens) != 2 {
		t.Fatalf("expected 2 tokens, got: %d", len(tokens))
	}

	err = store.Revoke("token-1")
	if err != nil {
		t.Fatalf("failed to revoke token-1: %v", err)
	}

	tokens = store.List()
	if len(tokens) != 1 {
		t.Fatalf("expected 1 token after revoke, got: %d", len(tokens))
	}

	if tokens[0].Name != "token-2" {
		t.Fatalf("expected remaining token to be 'token-2', got: %s", tokens[0].Name)
	}
}

func TestHashToken(t *testing.T) {
	token := "sk_test_token_12345"
	hash1 := hashToken(token)
	hash2 := hashToken(token)

	if hash1 != hash2 {
		t.Fatal("expected same hash for same token")
	}

	if len(hash1) != 64 {
		t.Fatalf("expected 64 character hex hash, got: %d", len(hash1))
	}

	differentHash := hashToken("sk_different_token")
	if hash1 == differentHash {
		t.Fatal("expected different hash for different token")
	}
}

func TestGenerateSecureToken(t *testing.T) {
	token1, err := generateSecureToken()
	if err != nil {
		t.Fatalf("failed to generate token: %v", err)
	}

	token2, err := generateSecureToken()
	if err != nil {
		t.Fatalf("failed to generate second token: %v", err)
	}

	if token1 == token2 {
		t.Fatal("expected unique tokens")
	}

	if token1[:3] != "sk_" {
		t.Fatalf("token should start with 'sk_', got: %s", token1[:3])
	}

	if len(token1) < 40 {
		t.Fatalf("token too short: %d characters", len(token1))
	}
}

func TestTokenStore_FilePermissions(t *testing.T) {
	tmpDir := t.TempDir()
	storePath := filepath.Join(tmpDir, "tokens.json")

	store, err := NewTokenStore(storePath)
	if err != nil {
		t.Fatalf("failed to create token store: %v", err)
	}

	_, err = store.Create("test-token")
	if err != nil {
		t.Fatalf("failed to create token: %v", err)
	}

	info, err := os.Stat(storePath)
	if err != nil {
		t.Fatalf("failed to stat token file: %v", err)
	}

	perm := info.Mode().Perm()
	if perm != 0600 {
		t.Fatalf("expected file permissions 0600, got: %o", perm)
	}
}

