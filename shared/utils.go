package shared

import (
	"crypto/rand"
	"errors"
	"math"
)

var defaultAlphabet = []rune("_-0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ")

const defaultSize = 8

func getMask(alphabetSize int) int {
	for i := 1; i <= 8; i++ {
		mask := (2 << uint(i)) - 1
		if mask >= alphabetSize-1 {
			return mask
		}
	}
	return 0
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

	logger.Debug().Int("alphabet_length", len(alphabet)).Int("id_size", size).Msg("generating nano ID with custom alphabet")

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
					logger.Debug().Str("generated_id", generatedID).Msg("successfully generated nano ID")
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

	logger.Debug().Int("id_length", size).Msg("generating standard nano ID")

	bytes := make([]byte, size)
	_, err := rand.Read(bytes)
	if err != nil {
		logger.Error().Err(err).Msg("failed to read random bytes for standard ID generation")
		return "", err
	}

	id := make([]rune, size)
	for i := 0; i < size; i++ {
		id[i] = defaultAlphabet[bytes[i]&63]
	}

	generatedID := string(id[:size])
	logger.Debug().Str("generated_id", generatedID).Msg("successfully generated standard nano ID")
	return generatedID, nil
}

func MustGenerateNanoID(length ...int) string {
	logger := GetLogger("shared.utils")

	id, err := GenerateNanoID(length...)
	if err != nil {
		logger.Fatal().Err(err).Msg("critical failure in ID generation - panicking")
		panic(err)
	}
	return id
}
