package main

import (
	"flag"
	"os"
	"os/signal"
	"syscall"

	"tunneling/client"
	"tunneling/shared"
)

func main() {
	server := flag.String("server", "http://localhost:8080", "tunnel server url")
	local := flag.String("local", "localhost:3000", "local address to tunnel")
	id := flag.String("id", "", "tunnel id (subdomain)")
	flag.Parse()

	shared.InitializeLogging(shared.DefaultLogConfig())

	logger := shared.GetLogger("client")

	logger.Info().
		Str("version", "1.0.0").
		Str("server", *server).
		Str("local", *local).
		Msg("tunnel client starting up")

	if id == nil || *id == "" {
		*id = shared.MustGenerateNanoID()
		logger.Info().Str("generated_id", *id).Msg("generated tunnel ID")
	} else {
		logger.Info().Str("provided_id", *id).Msg("using provided tunnel ID")
	}

	if *server == "" {
		logger.Fatal().Msg("server URL cannot be empty")
	}
	if *local == "" {
		logger.Fatal().Msg("local address cannot be empty")
	}

	c := &client.Client{
		ServerURL: *server,
		LocalAddr: *local,
		TunnelID:  *id,
	}

	logger.Info().Msg("starting tunnel client")
	logger.Info().Str("server", c.ServerURL).Msg("server url")
	logger.Info().Str("local", c.LocalAddr).Msg("local address")
	logger.Info().Str("tunnel_id", c.TunnelID).Msg("tunnel id")

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		sig := <-sigChan
		logger.Info().Str("signal", sig.String()).Msg("received shutdown signal")
		logger.Info().Msg("client shutting down")
		os.Exit(0)
	}()

	logger.Info().Msg("client configuration validated, starting tunnel")
	c.Run()
}
