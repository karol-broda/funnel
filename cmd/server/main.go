// Package main provides the main entry point for the funnel server.
//
// @title           Funnel Server API
// @version         1.0
// @description     REST API for tunnel management and monitoring in the funnel tunneling solution
// @description
// @description     This API provides endpoints for:
// @description     - Health checking
// @description     - Server statistics and monitoring
// @description     - Tunnel management (list, view, delete)
// @description     - Individual tunnel statistics
//
// @contact.name   Funnel Support
// @contact.url    https://github.com/karol-broda/funnel
//
// @license.name  MIT
// @license.url   https://github.com/karol-broda/funnel/blob/master/LICENSE.md
//
// @host      localhost:8080
// @BasePath  /api
//
// @schemes   http https
//
// @tag.name Server
// @tag.description Server health and statistics endpoints
//
// @tag.name Tunnels
// @tag.description Tunnel management and monitoring endpoints
package main

import (
	"crypto/tls"
	"fmt"
	"net/http"
	"os"
	"runtime"
	"time"

	"github.com/karol-broda/funnel/server"
	"github.com/karol-broda/funnel/shared"
	"github.com/karol-broda/funnel/version"
	"github.com/spf13/cobra"
)

var (
	port               int
	tlsPort            int
	host               string
	enableTls          bool
	certDir            string
	letsEncryptEmail   string
	dnsProvidersConfig string
	tokenStorePath     string
	requireAuth        bool
)

func getDefaultCertDir() string {
	if runtime.GOOS == "linux" || runtime.GOOS == "darwin" || runtime.GOOS == "freebsd" {
		return "/var/lib/funnel/certs"
	}
	return "./certs"
}

func getDefaultTokenStorePath() string {
	if runtime.GOOS == "linux" || runtime.GOOS == "darwin" || runtime.GOOS == "freebsd" {
		return "/var/lib/funnel/tokens.json"
	}
	return "./tokens.json"
}

func main() {
	rootCmd := &cobra.Command{
		Use:   "server",
		Short: "funnel server",
		Run:   runServer,
	}
	versionCmd := &cobra.Command{
		Use:   "version",
		Short: "print version information",
		Run: func(cmd *cobra.Command, args []string) {
			version.PrintVersionInfo("tunnel-server")
		},
	}

	tokenCmd := &cobra.Command{
		Use:   "token",
		Short: "manage authentication tokens",
	}

	tokenCreateCmd := &cobra.Command{
		Use:   "create",
		Short: "create a new authentication token",
		Run:   runTokenCreate,
	}
	tokenCreateCmd.Flags().StringVar(&tokenName, "name", "", "name for the token (required)")
	tokenCreateCmd.MarkFlagRequired("name")

	tokenListCmd := &cobra.Command{
		Use:   "list",
		Short: "list all active tokens",
		Run:   runTokenList,
	}

	tokenRevokeCmd := &cobra.Command{
		Use:   "revoke",
		Short: "revoke a token",
		Run:   runTokenRevoke,
	}
	tokenRevokeCmd.Flags().StringVar(&tokenName, "name", "", "name of the token to revoke (required)")
	tokenRevokeCmd.MarkFlagRequired("name")

	tokenCmd.AddCommand(tokenCreateCmd, tokenListCmd, tokenRevokeCmd)
	tokenCmd.PersistentFlags().StringVar(&tokenStorePath, "token-store", getDefaultTokenStorePath(), "path to token store file")

	rootCmd.PersistentFlags().IntVarP(&port, "port", "p", 8080, "port to listen on for http")
	rootCmd.PersistentFlags().IntVar(&tlsPort, "tls-port", 8443, "port to listen on for https")
	rootCmd.PersistentFlags().StringVar(&host, "host", "0.0.0.0", "host to listen on")
	rootCmd.PersistentFlags().BoolVar(&enableTls, "enable-tls", false, "enable tls with let's encrypt")
	rootCmd.PersistentFlags().StringVar(&certDir, "cert-dir", getDefaultCertDir(), "directory to store tls certificates")
	rootCmd.PersistentFlags().StringVar(&letsEncryptEmail, "letsencrypt-email", "", "email address for let's encrypt")
	rootCmd.PersistentFlags().StringVar(&dnsProvidersConfig, "dns-providers-config", "", "path to dns providers config file")
	rootCmd.PersistentFlags().StringVar(&tokenStorePath, "token-store", getDefaultTokenStorePath(), "path to token store file")
	rootCmd.PersistentFlags().BoolVar(&requireAuth, "require-auth", false, "require authentication for tunnel connections")
	rootCmd.AddCommand(versionCmd, tokenCmd)

	if err := rootCmd.Execute(); err != nil {
		fmt.Println(err)
		os.Exit(1)
	}
}

var tokenName string

func runTokenCreate(cmd *cobra.Command, args []string) {
	shared.InitializeLogging(shared.DefaultLogConfig())

	storePath := cmd.Flag("token-store").Value.String()
	tokenStore, err := server.NewTokenStore(storePath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: failed to load token store: %v\n", err)
		os.Exit(1)
	}

	plainToken, err := tokenStore.Create(tokenName)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: failed to create token: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("\nToken created for %q\n\n", tokenName)
	fmt.Printf("  %s\n\n", plainToken)
	fmt.Printf("  Save this token now - it will NOT be shown again.\n")
	fmt.Printf("  Token is persisted to disk and survives server restarts.\n\n")
}

func runTokenList(cmd *cobra.Command, args []string) {
	shared.InitializeLogging(shared.DefaultLogConfig())

	storePath := cmd.Flag("token-store").Value.String()
	tokenStore, err := server.NewTokenStore(storePath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: failed to load token store: %v\n", err)
		os.Exit(1)
	}

	tokens := tokenStore.List()
	if len(tokens) == 0 {
		fmt.Println("No active tokens found.")
		return
	}

	fmt.Printf("\n%-20s %-12s %s\n", "NAME", "PREFIX", "CREATED")
	fmt.Printf("%-20s %-12s %s\n", "----", "------", "-------")
	for _, t := range tokens {
		age := formatAge(t.CreatedAt)
		fmt.Printf("%-20s %-12s %s\n", t.Name, t.Prefix+"...", age)
	}
	fmt.Println()
}

func runTokenRevoke(cmd *cobra.Command, args []string) {
	shared.InitializeLogging(shared.DefaultLogConfig())

	storePath := cmd.Flag("token-store").Value.String()
	tokenStore, err := server.NewTokenStore(storePath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: failed to load token store: %v\n", err)
		os.Exit(1)
	}

	if err := tokenStore.Revoke(tokenName); err != nil {
		fmt.Fprintf(os.Stderr, "error: failed to revoke token: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("Token %q revoked.\n", tokenName)
}

func formatAge(t time.Time) string {
	d := time.Since(t)
	switch {
	case d < time.Minute:
		return "just now"
	case d < time.Hour:
		return fmt.Sprintf("%d minutes ago", int(d.Minutes()))
	case d < 24*time.Hour:
		return fmt.Sprintf("%d hours ago", int(d.Hours()))
	case d < 7*24*time.Hour:
		return fmt.Sprintf("%d days ago", int(d.Hours()/24))
	default:
		return t.Format("2006-01-02")
	}
}

func runServer(cmd *cobra.Command, args []string) {
	logger := shared.GetLogger("server.main")

	s := server.NewServer()
	tunnelRouter := server.NewTunnelRouter(s)
	s.SetRouter(tunnelRouter)

	// Initialize API handler
	apiHandler := server.NewAPIHandler(s, tunnelRouter)
	tunnelRouter.SetAPIHandler(apiHandler)

	// Initialize token store if authentication is enabled
	if requireAuth {
		tokenStore, err := server.NewTokenStore(tokenStorePath)
		if err != nil {
			logger.Fatal().Err(err).Str("path", tokenStorePath).Msg("failed to initialize token store")
		}
		s.SetTokenStore(tokenStore)

		if tokenStore.Count() == 0 {
			logger.Warn().Msg("authentication enabled but no tokens exist - run 'server token create --name <name>' to create one")
		}
	} else {
		logger.Warn().Msg("authentication disabled - anyone can create tunnels. use --require-auth to enable")
	}

	if enableTls {
		if letsEncryptEmail == "" || dnsProvidersConfig == "" {
			logger.Fatal().Msg("--letsencrypt-email and --dns-providers-config must be set when --enable-tls is true")
		}

		logger.Info().
			Str("email", letsEncryptEmail).
			Str("cert_dir", certDir).
			Str("dns_config", dnsProvidersConfig).
			Msg("tls is enabled, initializing certificate manager")

		certManager, err := server.NewCertificateManager(letsEncryptEmail, certDir, dnsProvidersConfig)
		if err != nil {
			logger.Fatal().Err(err).Msg("failed to initialize certificate manager")
		}

		if err := certManager.PreloadCertificates(); err != nil {
			logger.Fatal().Err(err).Msg("failed to preload certificates")
		}

		tlsConfig := &tls.Config{
			GetCertificate: certManager.GetCertificate,
			MinVersion:     tls.VersionTLS12,
		}

		httpsAddr := fmt.Sprintf("%s:%d", host, tlsPort)
		httpsServer := &http.Server{
			Addr:      httpsAddr,
			Handler:   tunnelRouter,
			TLSConfig: tlsConfig,
		}

		logger.Info().Str("address", httpsAddr).Msg("starting https server")
		go func() {
			if err := httpsServer.ListenAndServeTLS("", ""); err != nil && err != http.ErrServerClosed {
				logger.Fatal().Err(err).Msg("https server failed")
			}
		}()

		httpAddr := fmt.Sprintf("%s:%d", host, port)
		httpServer := &http.Server{
			Addr: httpAddr,
			Handler: http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
				http.Redirect(w, r, "https://"+r.Host+r.URL.String(), http.StatusMovedPermanently)
			}),
		}
		logger.Info().Str("address", httpAddr).Msg("starting http to https redirect server")
		go func() {
			if err := httpServer.ListenAndServe(); err != nil && err != http.ErrServerClosed {
				logger.Fatal().Err(err).Msg("http redirect server failed")
			}
		}()

	} else {
		httpAddr := fmt.Sprintf("%s:%d", host, port)
		httpServer := &http.Server{
			Addr:    httpAddr,
			Handler: tunnelRouter,
		}
		logger.Info().Str("address", httpAddr).Msg("starting http server")
		go func() {
			if err := httpServer.ListenAndServe(); err != nil && err != http.ErrServerClosed {
				logger.Fatal().Err(err).Msg("http server failed")
			}
		}()
	}

	// wait indefinitely
	select {}
}
