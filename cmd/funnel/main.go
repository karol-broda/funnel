package main

import (
	"fmt"
	"os"
	"os/signal"
	"path/filepath"
	"strings"
	"syscall"

	"github.com/karol-broda/funnel/client"
	"github.com/karol-broda/funnel/shared"
	"github.com/karol-broda/funnel/version"
	"github.com/rs/zerolog"
	"github.com/spf13/cobra"
)

var (
	server string
	local  string
	id     string
	inlet  string
	token  string
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

var configCmd = &cobra.Command{
	Use:   "config",
	Short: "manage client configuration",
}

var configSetTokenCmd = &cobra.Command{
	Use:   "set-token <token>",
	Short: "save authentication token to config file",
	Args:  cobra.ExactArgs(1),
	Run:   runConfigSetToken,
}

var configSetServerCmd = &cobra.Command{
	Use:   "set-server <url>",
	Short: "save server URL to config file",
	Args:  cobra.ExactArgs(1),
	Run:   runConfigSetServer,
}

var configShowCmd = &cobra.Command{
	Use:   "show",
	Short: "show current configuration",
	Run:   runConfigShow,
}

var configPathCmd = &cobra.Command{
	Use:   "path",
	Short: "show config file path",
	Run:   runConfigPath,
}

var configInlet string

func init() {
	httpCmd.Flags().StringVarP(&server, "server", "s", "", "tunnel server url (overrides config)")
	httpCmd.Flags().StringVarP(&id, "id", "i", "", "tunnel id (subdomain)")
	httpCmd.Flags().StringVarP(&inlet, "inlet", "", "default", "inlet configuration to use")
	httpCmd.Flags().StringVarP(&token, "token", "t", "", "authentication token (overrides config)")

	configCmd.PersistentFlags().StringVar(&configInlet, "inlet", "default", "inlet to configure")
	configCmd.AddCommand(configSetTokenCmd, configSetServerCmd, configShowCmd, configPathCmd)

	rootCmd.AddCommand(httpCmd)
	rootCmd.AddCommand(versionCmd)
	rootCmd.AddCommand(configCmd)
}

func runHTTPClient(cmd *cobra.Command, args []string) {
	shared.InitializeLogging(shared.DefaultLogConfig())

	logger := shared.GetLogger("client")

	logger.Info().
		Str("version", version.GetVersion()).
		Str("inlet", inlet).
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

	// resolve server URL and token from config or command line flags
	finalServer, finalToken, err := resolveServerAndToken(logger)
	if err != nil {
		logger.Fatal().Err(err).Msg("failed to resolve server configuration")
	}

	logger.Info().Str("server", finalServer).Bool("has_token", finalToken != "").Msg("using server configuration")

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

	client.Run(id, finalServer, local, finalToken, shutdownChan)

	logger.Info().Msg("client has shut down")
}

func resolveServerAndToken(logger zerolog.Logger) (string, string, error) {
	var finalServer string
	var finalToken string

	// try to load configuration
	configManager := client.NewConfigManager()
	inletConfig, err := configManager.GetInlet(inlet)

	// resolve server: command line flag takes precedence over config
	if server != "" {
		finalServer = server
		logger.Info().Str("server", server).Msg("using server from command line flag")
	} else if err == nil && inletConfig != nil {
		finalServer = inletConfig.Server
		logger.Info().
			Str("inlet", inlet).
			Str("server", inletConfig.Server).
			Str("domain", inletConfig.Domain).
			Msg("using server from configuration")
	} else {
		logger.Error().Err(err).Str("inlet", inlet).Msg("failed to load inlet configuration")
		return "", "", fmt.Errorf("no configuration found. please specify --server flag or create a config file at ~/.config/funnel/config.toml")
	}

	// resolve token: command line flag takes precedence over config
	if token != "" {
		finalToken = token
		logger.Info().Msg("using token from command line flag")
	} else if err == nil && inletConfig != nil && inletConfig.Token != "" {
		finalToken = inletConfig.Token
		logger.Info().Str("inlet", inlet).Msg("using token from configuration")
	}

	return finalServer, finalToken, nil
}

func runConfigSetToken(cmd *cobra.Command, args []string) {
	tokenValue := args[0]

	configManager := client.NewConfigManager()
	if err := configManager.SetToken(configInlet, tokenValue); err != nil {
		fmt.Fprintf(os.Stderr, "error: failed to save token: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Token saved to inlet %q\n", configInlet)
	fmt.Printf("  config file: %s\n", configManager.GetConfigPath())
	fmt.Printf("\n  Warning: token is stored in plain text in the config file.\n")
	fmt.Printf("  Ensure the file has appropriate permissions (chmod 600).\n")
}

func runConfigSetServer(cmd *cobra.Command, args []string) {
	serverValue := args[0]

	configManager := client.NewConfigManager()
	if err := configManager.SetServer(configInlet, serverValue); err != nil {
		fmt.Fprintf(os.Stderr, "error: failed to save server: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Server saved to inlet %q\n", configInlet)
	fmt.Printf("  config file: %s\n", configManager.GetConfigPath())
}

func runConfigShow(cmd *cobra.Command, args []string) {
	configManager := client.NewConfigManager()
	config := configManager.GetConfig()

	fmt.Printf("Config file: %s\n\n", configManager.GetConfigPath())

	if config == nil || len(config.Inlets) == 0 {
		fmt.Println("No inlets configured.")
		return
	}

	for name, inlet := range config.Inlets {
		fmt.Printf("[%s]\n", name)
		if inlet.Server != "" {
			fmt.Printf("  server: %s\n", inlet.Server)
		}
		if inlet.Domain != "" {
			fmt.Printf("  domain: %s\n", inlet.Domain)
		}
		if inlet.Token != "" {
			fmt.Printf("  token:  %s...\n", inlet.Token[:min(10, len(inlet.Token))])
		}
		fmt.Println()
	}
}

func runConfigPath(cmd *cobra.Command, args []string) {
	configManager := client.NewConfigManager()
	fmt.Println(configManager.GetConfigPath())
}

func main() {
	if err := rootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}
