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
