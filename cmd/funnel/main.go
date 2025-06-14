package main

import (
	"os"
	"os/signal"
	"path/filepath"
	"strings"
	"syscall"

	"github.com/karol-broda/funnel/client"
	"github.com/karol-broda/funnel/shared"
	"github.com/karol-broda/funnel/version"
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
}

var httpCmd = &cobra.Command{
	Use:   "http [address:port | port]",
	Short: "create an HTTP tunnel",
	Long:  `create an HTTP tunnel to expose a local HTTP service through a remote server`,
	Args:  cobra.ExactArgs(1),
	Run:   runHTTPClient,
}

var versionCmd = &cobra.Command{
	Use:   "version",
	Short: "print version information",
	Run: func(cmd *cobra.Command, args []string) {
		version.PrintVersionInfo("tunnel-client")
	},
}

func init() {
	httpCmd.Flags().StringVarP(&server, "server", "s", "http://localhost:8080", "tunnel server url")
	httpCmd.Flags().StringVarP(&id, "id", "i", "", "tunnel id (subdomain)")

	rootCmd.AddCommand(httpCmd)
	rootCmd.AddCommand(versionCmd)
}

func runHTTPClient(cmd *cobra.Command, args []string) {
	shared.InitializeLogging(shared.DefaultLogConfig())

	logger := shared.GetLogger("client")

	logger.Info().
		Str("version", version.GetVersion()).
		Str("server", server).
		Msg("tunnel client starting up")

	localArg := args[0]
	if localArg == "" {
		logger.Fatal().Msg("local address or port cannot be empty")
	}

	if strings.Contains(localArg, ":") {
		local = localArg
		logger.Info().Str("local_address", local).Msg("using provided address:port")
	} else {
		local = "localhost:" + localArg
		logger.Info().Str("port", localArg).Str("constructed_local", local).Msg("constructed local address from port")
	}

	if id == "" {
		id = shared.MustGenerateDomainSafeID()
		logger.Info().Str("generated_id", id).Msg("generated domain-safe tunnel ID")
	} else {
		if err := shared.ValidateTunnelID(id); err != nil {
			logger.Fatal().Err(err).Str("provided_id", id).Msg("invalid tunnel ID provided")
		}
		logger.Info().Str("provided_id", id).Msg("using provided tunnel ID")
	}

	if server == "" {
		logger.Fatal().Msg("server URL cannot be empty")
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
