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
		version.PrintVersionInfo("funnel-client")
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

	if server == "" {
		logger.Fatal().Msg("server URL cannot be empty")
	}

	if id == "" {
		generatedID, err := shared.GenerateDomainSafeID()
		if err != nil {
			logger.Fatal().Err(err).Msg("failed to generate tunnel ID")
		}
		id = generatedID
		logger.Info().Str("generated_id", id).Msg("generated tunnel ID")
	} else {
		logger.Info().Str("provided_id", id).Msg("using provided tunnel ID")
	}

	logger.Info().Msg("client configuration validated, starting tunnel")

	shutdownChan := make(chan struct{})
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		<-sigChan
		logger.Info().Msg("received shutdown signal, initiating shutdown")
		close(shutdownChan)
	}()

	client.Run(id, server, local, shutdownChan)

	logger.Info().Msg("client has shut down")
}

func main() {
	if err := rootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
