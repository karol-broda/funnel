package shared

import (
	"regexp"
	"strings"
	"testing"
)

func TestGenerateNanoID(t *testing.T) {
	tests := []struct {
		name        string
		length      []int
		expectedLen int
		shouldError bool
	}{
		{
			name:        "default length",
			length:      []int{},
			expectedLen: 8,
			shouldError: false,
		},
		{
			name:        "custom length 12",
			length:      []int{12},
			expectedLen: 12,
			shouldError: false,
		},
		{
			name:        "zero length",
			length:      []int{0},
			expectedLen: 0,
			shouldError: false,
		},
		{
			name:        "negative length",
			length:      []int{-1},
			expectedLen: 0,
			shouldError: true,
		},
		{
			name:        "too many parameters",
			length:      []int{8, 10},
			expectedLen: 0,
			shouldError: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			id, err := GenerateNanoID(tt.length...)

			if tt.shouldError {
				if err == nil {
					t.Errorf("expected error but got none")
				}
				return
			}

			if err != nil {
				t.Errorf("unexpected error: %v", err)
				return
			}

			if len(id) != tt.expectedLen {
				t.Errorf("expected length %d, got %d", tt.expectedLen, len(id))
			}

			// verify characters are from the default alphabet
			validChars := regexp.MustCompile(`^[_\-0-9a-zA-Z]*$`)
			if !validChars.MatchString(id) {
				t.Errorf("generated ID contains invalid characters: %s", id)
			}
		})
	}
}

func TestGenerateNanoIDUniqueness(t *testing.T) {
	generated := make(map[string]bool)
	iterations := 1000

	for i := 0; i < iterations; i++ {
		id, err := GenerateNanoID()
		if err != nil {
			t.Fatalf("unexpected error generating ID: %v", err)
		}

		if generated[id] {
			t.Errorf("duplicate ID generated: %s", id)
		}
		generated[id] = true
	}

	if len(generated) != iterations {
		t.Errorf("expected %d unique IDs, got %d", iterations, len(generated))
	}
}

func TestGenerateNanoIDWithAlphabet(t *testing.T) {
	tests := []struct {
		name        string
		alphabet    string
		size        int
		shouldError bool
		errorMsg    string
	}{
		{
			name:        "valid alphabet and size",
			alphabet:    "abc123",
			size:        8,
			shouldError: false,
		},
		{
			name:        "single character alphabet",
			alphabet:    "a",
			size:        5,
			shouldError: false,
		},
		{
			name:        "empty alphabet",
			alphabet:    "",
			size:        8,
			shouldError: true,
			errorMsg:    "alphabet must not be empty",
		},
		{
			name:        "too large alphabet",
			alphabet:    strings.Repeat("a", 256),
			size:        8,
			shouldError: true,
			errorMsg:    "alphabet must not be empty and contain no more than 255 chars",
		},
		{
			name:        "zero size",
			alphabet:    "abc",
			size:        0,
			shouldError: true,
			errorMsg:    "size must be positive integer",
		},
		{
			name:        "negative size",
			alphabet:    "abc",
			size:        -1,
			shouldError: true,
			errorMsg:    "size must be positive integer",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			id, err := GenerateNanoIDWithAlphabet(tt.alphabet, tt.size)

			if tt.shouldError {
				if err == nil {
					t.Errorf("expected error but got none")
					return
				}
				if !strings.Contains(err.Error(), tt.errorMsg) {
					t.Errorf("expected error containing '%s', got '%s'", tt.errorMsg, err.Error())
				}
				return
			}

			if err != nil {
				t.Errorf("unexpected error: %v", err)
				return
			}

			if len(id) != tt.size {
				t.Errorf("expected length %d, got %d", tt.size, len(id))
			}

			// verify all characters are from the provided alphabet
			for _, char := range id {
				if !strings.ContainsRune(tt.alphabet, char) {
					t.Errorf("generated ID contains character '%c' not in alphabet '%s'", char, tt.alphabet)
				}
			}
		})
	}
}

func TestGenerateNanoIDWithAlphabetUniqueness(t *testing.T) {
	alphabet := "abcdef123456"
	size := 8
	generated := make(map[string]bool)
	iterations := 500

	for i := 0; i < iterations; i++ {
		id, err := GenerateNanoIDWithAlphabet(alphabet, size)
		if err != nil {
			t.Fatalf("unexpected error generating ID: %v", err)
		}

		if generated[id] {
			t.Errorf("duplicate ID generated: %s", id)
		}
		generated[id] = true
	}
}

func TestMustGenerateNanoID(t *testing.T) {
	t.Run("successful generation", func(t *testing.T) {
		defer func() {
			if r := recover(); r != nil {
				t.Errorf("unexpected panic: %v", r)
			}
		}()

		id := MustGenerateNanoID(10)
		if len(id) != 10 {
			t.Errorf("expected length 10, got %d", len(id))
		}
	})

	t.Run("panic on error", func(t *testing.T) {
		defer func() {
			if r := recover(); r == nil {
				t.Errorf("expected panic but got none")
			}
		}()

		// this should panic due to negative length
		MustGenerateNanoID(-1)
	})
}

func TestGetMask(t *testing.T) {
	tests := []struct {
		alphabetSize int
		expected     int
	}{
		{1, 1},
		{2, 1},
		{3, 3},
		{4, 3},
		{8, 7},
		{16, 15},
		{32, 31},
		{64, 63},
		{128, 127},
		{256, 255},
	}

	for _, tt := range tests {
		t.Run("", func(t *testing.T) {
			result := getMask(tt.alphabetSize)
			if result != tt.expected {
				t.Errorf("getMask(%d) = %d, want %d", tt.alphabetSize, result, tt.expected)
			}
		})
	}
}

// benchmark tests
func BenchmarkGenerateNanoID(b *testing.B) {
	b.Run("default length", func(b *testing.B) {
		b.ReportAllocs()
		for i := 0; i < b.N; i++ {
			_, err := GenerateNanoID()
			if err != nil {
				b.Fatal(err)
			}
		}
	})

	b.Run("length 16", func(b *testing.B) {
		b.ReportAllocs()
		for i := 0; i < b.N; i++ {
			_, err := GenerateNanoID(16)
			if err != nil {
				b.Fatal(err)
			}
		}
	})
}

func BenchmarkGenerateNanoIDWithAlphabet(b *testing.B) {
	alphabet := "0123456789abcdef"

	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		_, err := GenerateNanoIDWithAlphabet(alphabet, 8)
		if err != nil {
			b.Fatal(err)
		}
	}
}

func BenchmarkMustGenerateNanoID(b *testing.B) {
	b.ReportAllocs()
	for i := 0; i < b.N; i++ {
		_ = MustGenerateNanoID()
	}
}

func TestValidateTunnelID(t *testing.T) {
	tests := []struct {
		name    string
		id      string
		wantErr bool
		errMsg  string
	}{
		{
			name:    "valid lowercase alphanumeric",
			id:      "abc123",
			wantErr: false,
		},
		{
			name:    "valid with hyphens",
			id:      "abc-123-def",
			wantErr: false,
		},
		{
			name:    "valid minimum length",
			id:      "abc",
			wantErr: false,
		},
		{
			name:    "valid maximum length",
			id:      strings.Repeat("a", 63),
			wantErr: false,
		},
		{
			name:    "empty string",
			id:      "",
			wantErr: true,
			errMsg:  "tunnel ID cannot be empty",
		},
		{
			name:    "too short",
			id:      "ab",
			wantErr: true,
			errMsg:  "tunnel ID must be at least 3 characters long",
		},
		{
			name:    "too long",
			id:      strings.Repeat("a", 64),
			wantErr: true,
			errMsg:  "tunnel ID must be no more than 63 characters long",
		},
		{
			name:    "contains uppercase",
			id:      "Abc123",
			wantErr: true,
			errMsg:  "tunnel ID must be lowercase",
		},
		{
			name:    "starts with hyphen",
			id:      "-abc123",
			wantErr: true,
			errMsg:  "tunnel ID must contain only lowercase letters, numbers, and hyphens, and cannot start or end with a hyphen",
		},
		{
			name:    "ends with hyphen",
			id:      "abc123-",
			wantErr: true,
			errMsg:  "tunnel ID must contain only lowercase letters, numbers, and hyphens, and cannot start or end with a hyphen",
		},
		{
			name:    "contains invalid characters",
			id:      "abc_123",
			wantErr: true,
			errMsg:  "tunnel ID must contain only lowercase letters, numbers, and hyphens, and cannot start or end with a hyphen",
		},
		{
			name:    "contains spaces",
			id:      "abc 123",
			wantErr: true,
			errMsg:  "tunnel ID must contain only lowercase letters, numbers, and hyphens, and cannot start or end with a hyphen",
		},
		{
			name:    "contains special characters",
			id:      "abc@123",
			wantErr: true,
			errMsg:  "tunnel ID must contain only lowercase letters, numbers, and hyphens, and cannot start or end with a hyphen",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := ValidateTunnelID(tt.id)
			if tt.wantErr {
				if err == nil {
					t.Errorf("ValidateTunnelID() expected error but got none")
					return
				}
				if tt.errMsg != "" && !strings.Contains(err.Error(), tt.errMsg) {
					t.Errorf("ValidateTunnelID() error = %v, want error containing %v", err, tt.errMsg)
				}
			} else {
				if err != nil {
					t.Errorf("ValidateTunnelID() unexpected error = %v", err)
				}
			}
		})
	}
}

func TestGenerateDomainSafeID(t *testing.T) {
	tests := []struct {
		name       string
		length     []int
		wantErr    bool
		wantLength int
	}{
		{
			name:       "default length",
			length:     []int{},
			wantErr:    false,
			wantLength: 8,
		},
		{
			name:       "custom length 10",
			length:     []int{10},
			wantErr:    false,
			wantLength: 10,
		},
		{
			name:       "minimum length 3",
			length:     []int{3},
			wantErr:    false,
			wantLength: 3,
		},
		{
			name:       "maximum length 63",
			length:     []int{63},
			wantErr:    false,
			wantLength: 63,
		},
		{
			name:    "too short",
			length:  []int{2},
			wantErr: true,
		},
		{
			name:    "too long",
			length:  []int{64},
			wantErr: true,
		},
		{
			name:    "too many parameters",
			length:  []int{8, 10},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			id, err := GenerateDomainSafeID(tt.length...)
			if tt.wantErr {
				if err == nil {
					t.Errorf("GenerateDomainSafeID() expected error but got none")
				}
				return
			}

			if err != nil {
				t.Errorf("GenerateDomainSafeID() unexpected error = %v", err)
				return
			}

			if len(id) != tt.wantLength {
				t.Errorf("GenerateDomainSafeID() length = %v, want %v", len(id), tt.wantLength)
			}

			// validate the generated ID
			if err := ValidateTunnelID(id); err != nil {
				t.Errorf("GenerateDomainSafeID() generated invalid ID: %v, error: %v", id, err)
			}

			// check that it only contains domain-safe characters
			for _, char := range id {
				if !((char >= 'a' && char <= 'z') || (char >= '0' && char <= '9')) {
					t.Errorf("GenerateDomainSafeID() generated ID contains invalid character: %c in %s", char, id)
				}
			}
		})
	}
}

func TestMustGenerateDomainSafeID(t *testing.T) {
	t.Run("successful generation", func(t *testing.T) {
		id := MustGenerateDomainSafeID()
		if len(id) != 8 {
			t.Errorf("MustGenerateDomainSafeID() length = %v, want 8", len(id))
		}

		if err := ValidateTunnelID(id); err != nil {
			t.Errorf("MustGenerateDomainSafeID() generated invalid ID: %v, error: %v", id, err)
		}
	})

	t.Run("custom length", func(t *testing.T) {
		id := MustGenerateDomainSafeID(12)
		if len(id) != 12 {
			t.Errorf("MustGenerateDomainSafeID(12) length = %v, want 12", len(id))
		}

		if err := ValidateTunnelID(id); err != nil {
			t.Errorf("MustGenerateDomainSafeID(12) generated invalid ID: %v, error: %v", id, err)
		}
	})
}

func TestDomainSafeIDUniqueness(t *testing.T) {
	// generate multiple IDs and ensure they're unique
	ids := make(map[string]bool)
	numIDs := 1000

	for i := 0; i < numIDs; i++ {
		id := MustGenerateDomainSafeID()
		if ids[id] {
			t.Errorf("GenerateDomainSafeID() generated duplicate ID: %s", id)
		}
		ids[id] = true
	}

	if len(ids) != numIDs {
		t.Errorf("Expected %d unique IDs, got %d", numIDs, len(ids))
	}
}

func TestBackwardCompatibilityNanoID(t *testing.T) {
	// ensure the regular NanoID still works but now uses lowercase
	id, err := GenerateNanoID()
	if err != nil {
		t.Errorf("GenerateNanoID() unexpected error = %v", err)
	}

	if len(id) != 8 {
		t.Errorf("GenerateNanoID() length = %v, want 8", len(id))
	}

	// check that it doesn't contain uppercase letters anymore
	for _, char := range id {
		if char >= 'A' && char <= 'Z' {
			t.Errorf("GenerateNanoID() generated ID contains uppercase character: %c in %s", char, id)
		}
	}
}
