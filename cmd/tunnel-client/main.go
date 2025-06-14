package main

import (
	"os"
	"os/signal"
	"path/filepath"
	"syscall"

	"github.com/karol-broda/go-tunnel-proxy/client"
	"github.com/karol-broda/go-tunnel-proxy/shared"
	"github.com/karol-broda/go-tunnel-proxy/version"
	"github.com/spf13/cobra"
)

var (
	server string
	local  string
	id     string
)

var rootCmd = &cobra.Command{
	Use:   filepath.Base(os.Args[0]),
	Short: "a tunnel client for creating secure tunnels",
	Long:  `a tunnel client that creates secure tunnels to expose local services through a remote server`,
	Run:   runClient,
}

var versionCmd = &cobra.Command{
	Use:   "version",
	Short: "print version information",
	Run: func(cmd *cobra.Command, args []string) {
		version.PrintVersionInfo("tunnel-client")
	},
}

func init() {
	rootCmd.PersistentFlags().StringVarP(&server, "server", "s", "http://localhost:8080", "tunnel server url")
	rootCmd.PersistentFlags().StringVarP(&local, "local", "l", "localhost:3000", "local address to tunnel")
	rootCmd.PersistentFlags().StringVarP(&id, "id", "i", "", "tunnel id (subdomain)")
	rootCmd.AddCommand(versionCmd)
}

func runClient(cmd *cobra.Command, args []string) {
	shared.InitializeLogging(shared.DefaultLogConfig())

	logger := shared.GetLogger("client")

	logger.Info().
		Str("version", version.GetVersion()).
		Str("server", server).
		Str("local", local).
		Msg("tunnel client starting up")

	if id == "" {
		id = shared.MustGenerateNanoID()
		logger.Info().Str("generated_id", id).Msg("generated tunnel ID")
	} else {
		logger.Info().Str("provided_id", id).Msg("using provided tunnel ID")
	}

	if server == "" {
		logger.Fatal().Msg("server URL cannot be empty")
	}
	if local == "" {
		logger.Fatal().Msg("local address cannot be empty")
	}

	c := &client.Client{
		ServerURL: server,
		LocalAddr: local,
		TunnelID:  id,
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

func main() {
	if err := rootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
