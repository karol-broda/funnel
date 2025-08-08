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
)

func getDefaultCertDir() string {
	if runtime.GOOS == "linux" || runtime.GOOS == "darwin" || runtime.GOOS == "freebsd" {
		return "/var/lib/funnel/certs"
	}
	return "./certs"
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

	rootCmd.PersistentFlags().IntVarP(&port, "port", "p", 8080, "port to listen on for http")
	rootCmd.PersistentFlags().IntVar(&tlsPort, "tls-port", 8443, "port to listen on for https")
	rootCmd.PersistentFlags().StringVar(&host, "host", "0.0.0.0", "host to listen on")
	rootCmd.PersistentFlags().BoolVar(&enableTls, "enable-tls", false, "enable tls with let's encrypt")
	rootCmd.PersistentFlags().StringVar(&certDir, "cert-dir", getDefaultCertDir(), "directory to store tls certificates")
	rootCmd.PersistentFlags().StringVar(&letsEncryptEmail, "letsencrypt-email", "", "email address for let's encrypt")
	rootCmd.PersistentFlags().StringVar(&dnsProvidersConfig, "dns-providers-config", "", "path to dns providers config file")
	rootCmd.AddCommand(versionCmd)

	if err := rootCmd.Execute(); err != nil {
		fmt.Println(err)
		os.Exit(1)
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
