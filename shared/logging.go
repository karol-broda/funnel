package shared

import (
	"os"
	"time"

	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
)

type LogConfig struct {
	Level      string `json:"level"`
	Format     string `json:"format"`
	TimeFormat string `json:"time_format"`
}

func DefaultLogConfig() LogConfig {
	return LogConfig{
		Level:      "debug",
		Format:     "console",
		TimeFormat: time.RFC3339,
	}
}

func ProductionLogConfig() LogConfig {
	return LogConfig{
		Level:      "info",
		Format:     "json",
		TimeFormat: time.RFC3339,
	}
}

func InitializeLogging(config LogConfig) {
	level := zerolog.InfoLevel
	switch config.Level {
	case "debug":
		level = zerolog.DebugLevel
	case "info":
		level = zerolog.InfoLevel
	case "warn":
		level = zerolog.WarnLevel
	case "error":
		level = zerolog.ErrorLevel
	case "trace":
		level = zerolog.TraceLevel
	}

	if config.Format == "console" {
		log.Logger = log.Output(zerolog.ConsoleWriter{
			Out:        os.Stderr,
			TimeFormat: config.TimeFormat,
			NoColor:    false,
		}).Level(level).With().Timestamp().Caller().Logger()
	} else {
		log.Logger = zerolog.New(os.Stderr).
			Level(level).
			With().
			Timestamp().
			Caller().
			Logger()
	}
}

func GetLogger(component string) zerolog.Logger {
	return log.With().Str("component", component).Logger()
}

func GetTunnelLogger(component, tunnelID string) zerolog.Logger {
	return log.With().
		Str("component", component).
		Str("tunnel_id", tunnelID).
		Logger()
}

func GetRequestLogger(component, tunnelID, requestID string) zerolog.Logger {
	return log.With().
		Str("component", component).
		Str("tunnel_id", tunnelID).
		Str("request_id", requestID).
		Logger()
}
