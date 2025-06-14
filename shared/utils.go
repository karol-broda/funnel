package shared

import (
	"crypto/rand"
	"errors"
	"math"
	"regexp"
	"strings"
)

var defaultAlphabet = []rune("_-0123456789abcdefghijklmnopqrstuvwxyz")
var domainSafeAlphabet = []rune("0123456789abcdefghijklmnopqrstuvwxyz")

const defaultSize = 8

var tunnelIDRegex = regexp.MustCompile(`^[a-z0-9][a-z0-9-]*[a-z0-9]$`)

func getMask(alphabetSize int) int {
	for i := 1; i <= 8; i++ {
		mask := (1 << uint(i)) - 1
		if mask >= alphabetSize-1 {
			return mask
		}
	}
	return 0
}

func ValidateTunnelID(id string) error {
	logger := GetLogger("shared.utils")

	if len(id) == 0 {
		logger.Error().Msg("tunnel ID cannot be empty")
		return errors.New("tunnel ID cannot be empty")
	}

	if len(id) < 3 {
		logger.Error().Int("length", len(id)).Msg("tunnel ID too short")
		return errors.New("tunnel ID must be at least 3 characters long")
	}

	if len(id) > 63 {
		logger.Error().Int("length", len(id)).Msg("tunnel ID too long")
		return errors.New("tunnel ID must be no more than 63 characters long")
	}

	lowerID := strings.ToLower(id)
	if id != lowerID {
		logger.Error().Str("id", id).Str("expected", lowerID).Msg("tunnel ID contains uppercase letters")
		return errors.New("tunnel ID must be lowercase")
	}

	if !tunnelIDRegex.MatchString(id) {
		logger.Error().Str("id", id).Msg("tunnel ID contains invalid characters")
		return errors.New("tunnel ID must contain only lowercase letters, numbers, and hyphens, and cannot start or end with a hyphen")
	}

	return nil
}

func GenerateDomainSafeID(length ...int) (string, error) {
	logger := GetLogger("shared.utils")

	var size int
	switch {
	case len(length) == 0:
		size = defaultSize
	case len(length) == 1:
		size = length[0]
		if size < 3 {
			logger.Error().Int("requested_length", size).Msg("domain-safe ID length too short")
			return "", errors.New("domain-safe ID must be at least 3 characters long")
		}
		if size > 63 {
			logger.Error().Int("requested_length", size).Msg("domain-safe ID length too long")
			return "", errors.New("domain-safe ID must be no more than 63 characters long")
		}
	default:
		logger.Error().Int("param_count", len(length)).Msg("too many parameters for domain-safe ID generation")
		return "", errors.New("unexpected parameter")
	}

	return GenerateNanoIDWithAlphabet(string(domainSafeAlphabet), size)
}

func MustGenerateDomainSafeID(length ...int) string {
	logger := GetLogger("shared.utils")

	id, err := GenerateDomainSafeID(length...)
	if err != nil {
		logger.Error().Err(err).Msg("critical failure in domain-safe ID generation - panicking")
		panic(err)
	}
	return id
}

func GenerateNanoIDWithAlphabet(alphabet string, size int) (string, error) {
	logger := GetLogger("shared.utils")

	chars := []rune(alphabet)

	if len(alphabet) == 0 || len(alphabet) > 255 {
		logger.Error().Int("alphabet_length", len(alphabet)).Msg("invalid alphabet length for ID generation")
		return "", errors.New("alphabet must not be empty and contain no more than 255 chars")
	}
	if size <= 0 {
		logger.Error().Int("size", size).Msg("invalid size for ID generation")
		return "", errors.New("size must be positive integer")
	}

	mask := getMask(len(chars))
	ceilArg := 1.6 * float64(mask*size) / float64(len(alphabet))
	step := int(math.Ceil(ceilArg))

	id := make([]rune, size)
	bytes := make([]byte, step)
	for j := 0; ; {
		_, err := rand.Read(bytes)
		if err != nil {
			logger.Error().Err(err).Msg("failed to read random bytes for ID generation")
			return "", err
		}
		for i := 0; i < step; i++ {
			currByte := bytes[i] & byte(mask)
			if currByte < byte(len(chars)) {
				id[j] = chars[currByte]
				j++
				if j == size {
					generatedID := string(id[:size])
					return generatedID, nil
				}
			}
		}
	}
}

func GenerateNanoID(length ...int) (string, error) {
	logger := GetLogger("shared.utils")

	var size int
	switch {
	case len(length) == 0:
		size = defaultSize
	case len(length) == 1:
		size = length[0]
		if size < 0 {
			logger.Error().Int("requested_length", size).Msg("negative ID length requested")
			return "", errors.New("negative id length")
		}
	default:
		logger.Error().Int("param_count", len(length)).Msg("too many parameters for ID generation")
		return "", errors.New("unexpected parameter")
	}

	bytes := make([]byte, size)
	_, err := rand.Read(bytes)
	if err != nil {
		logger.Error().Err(err).Msg("failed to read random bytes for standard ID generation")
		return "", err
	}

	id := make([]rune, size)
	alphabetLen := len(defaultAlphabet)
	for i := 0; i < size; i++ {
		id[i] = defaultAlphabet[int(bytes[i])%alphabetLen]
	}

	generatedID := string(id[:size])
	return generatedID, nil
}

func MustGenerateNanoID(length ...int) string {
	logger := GetLogger("shared.utils")

	id, err := GenerateNanoID(length...)
	if err != nil {
		logger.Error().Err(err).Msg("critical failure in ID generation - panicking")
		panic(err)
	}
	return id
}
