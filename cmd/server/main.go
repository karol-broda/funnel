package main

import (
	"net/http"
	"os"
	"os/signal"
	"path/filepath"
	"syscall"
	"time"

	"github.com/karol-broda/funnel/server"
	"github.com/karol-broda/funnel/shared"
	"github.com/karol-broda/funnel/version"
	"github.com/spf13/cobra"
)

var (
	port string
)

var rootCmd = &cobra.Command{
	Use:   filepath.Base(os.Args[0]),
	Short: "a tunnel server for proxying connections",
	Long:  `a tunnel server that allows clients to create tunnels and proxy connections through them`,
	Run:   runServer,
}

var versionCmd = &cobra.Command{
	Use:   "version",
	Short: "print version information",
	Run: func(cmd *cobra.Command, args []string) {
		version.PrintVersionInfo("tunnel-server")
	},
}

func init() {
	rootCmd.PersistentFlags().StringVarP(&port, "port", "p", "8080", "server port")
	rootCmd.AddCommand(versionCmd)
}

func runServer(cmd *cobra.Command, args []string) {
	shared.InitializeLogging(shared.DefaultLogConfig())

	logger := shared.GetLogger("server")

	logger.Info().
		Str("version", version.GetVersion()).
		Str("port", port).
		Msg("tunnel server starting up")

	s := server.NewServer()
	tunnelRouter := server.NewTunnelRouter(s)

	s.SetRouter(tunnelRouter)

	logger.Info().Str("port", port).Msg("tunnel server starting")
	logger.Info().Str("tunnel_format", "<tunnel-id>.localhost:"+port).Msg("tunnels will be available at")

	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		sig := <-sigChan
		logger.Info().Str("signal", sig.String()).Msg("received shutdown signal")

		time.Sleep(2 * time.Second)
		logger.Info().Msg("server shutting down")
		os.Exit(0)
	}()

	serverAddr := ":" + port
	logger.Info().Str("address", serverAddr).Msg("starting HTTP server")

	if err := http.ListenAndServe(serverAddr, tunnelRouter); err != nil {
		logger.Fatal().Err(err).Str("address", serverAddr).Msg("server failed to start")
	}
}

func main() {
	if err := rootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
