package main

import (
	"flag"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/karol-broda/go-tunnel-proxy/server"
	"github.com/karol-broda/go-tunnel-proxy/shared"
	"github.com/karol-broda/go-tunnel-proxy/version"
)

func main() {
	port := flag.String("port", "8080", "server port")
	showVersion := flag.Bool("version", false, "show version information")
	flag.Parse()

	if *showVersion {
		version.PrintVersionInfo("tunnel-server")
		return
	}

	shared.InitializeLogging(shared.DefaultLogConfig())

	logger := shared.GetLogger("server")

	logger.Info().
		Str("version", version.GetVersion()).
		Str("port", *port).
		Msg("tunnel server starting up")

	s := server.NewServer()
	tunnelRouter := server.NewTunnelRouter(s)

	s.SetRouter(tunnelRouter)

	logger.Info().Str("port", *port).Msg("tunnel server starting")
	logger.Info().Str("tunnel_format", "<tunnel-id>.localhost:"+*port).Msg("tunnels will be available at")

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		sig := <-sigChan
		logger.Info().Str("signal", sig.String()).Msg("received shutdown signal")

		time.Sleep(2 * time.Second)
		logger.Info().Msg("server shutting down")
		os.Exit(0)
	}()

	serverAddr := ":" + *port
	logger.Info().Str("address", serverAddr).Msg("starting HTTP server")

	if err := http.ListenAndServe(serverAddr, tunnelRouter); err != nil {
		logger.Fatal().Err(err).Str("address", serverAddr).Msg("server failed to start")
	}
}
