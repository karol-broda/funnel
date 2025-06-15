package server

import (
	"crypto"
	"crypto/ecdsa"
	"crypto/elliptic"
	"crypto/rand"
	"crypto/tls"
	"crypto/x509"
	"encoding/json"
	"encoding/pem"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"github.com/go-acme/lego/v4/certcrypto"
	"github.com/go-acme/lego/v4/certificate"
	"github.com/go-acme/lego/v4/challenge"
	"github.com/go-acme/lego/v4/lego"
	"github.com/go-acme/lego/v4/providers/dns"
	"github.com/go-acme/lego/v4/registration"
	lru "github.com/hashicorp/golang-lru/v2"
	"github.com/karol-broda/funnel/shared"
	"github.com/rs/zerolog"
)

type ProviderConfig struct {
	Providers []DNSProviderConfig `json:"providers"`
}

type DNSProviderConfig struct {
	Name     string            `json:"name"`
	Provider string            `json:"provider"`
	Domains  []string          `json:"domains"`
	Config   map[string]string `json:"config"`
}

type User struct {
	Email        string
	Registration *registration.Resource
	key          crypto.PrivateKey
}

func (u *User) GetEmail() string {
	return u.Email
}

func (u *User) GetRegistration() *registration.Resource {
	return u.Registration
}

func (u *User) GetPrivateKey() crypto.PrivateKey {
	return u.key
}

type ProviderMux struct {
	providers map[string]challenge.Provider
	logger    zerolog.Logger
}

type providerCreator func(config DNSProviderConfig) (challenge.Provider, error)

var providerCreateMutex sync.Mutex

func NewProviderMux(config *ProviderConfig) (*ProviderMux, error) {
	creator := func(pconf DNSProviderConfig) (challenge.Provider, error) {
		providerCreateMutex.Lock()
		defer providerCreateMutex.Unlock()

		for key, value := range pconf.Config {
			os.Setenv(key, value)
		}

		defer func() {
			for key := range pconf.Config {
				os.Unsetenv(key)
			}
		}()

		return dns.NewDNSChallengeProviderByName(pconf.Provider)
	}
	return newProviderMuxWithCreator(config, creator)
}

func newProviderMuxWithCreator(config *ProviderConfig, create providerCreator) (*ProviderMux, error) {
	logger := shared.GetLogger("certs.provider-mux")

	mux := &ProviderMux{
		providers: make(map[string]challenge.Provider),
		logger:    logger,
	}

	for _, pconf := range config.Providers {
		logger.Info().
			Str("provider_name", pconf.Name).
			Str("provider_type", pconf.Provider).
			Strs("domains", pconf.Domains).
			Msg("configuring DNS provider")

		provider, err := create(pconf)
		if err != nil {
			return nil, fmt.Errorf("failed to create provider %s: %w", pconf.Name, err)
		}

		for _, domain := range pconf.Domains {
			normDomain := strings.ToLower(strings.TrimSpace(domain))
			if strings.HasPrefix(normDomain, "*.") {
				baseDomain := normDomain[2:]
				mux.providers[baseDomain] = provider
				mux.providers[normDomain] = provider
			} else {
				mux.providers[normDomain] = provider
			}

			logger.Debug().
				Str("domain", normDomain).
				Str("provider", pconf.Name).
				Msg("mapped domain to provider")
		}
	}

	logger.Info().
		Int("total_providers", len(config.Providers)).
		Int("total_domain_mappings", len(mux.providers)).
		Msg("provider multiplexer initialized")

	return mux, nil
}

func (m *ProviderMux) FindManagedDomain(domain string) (string, bool) {
	if _, ok := m.providers[domain]; ok {
		return domain, true
	}

	parts := strings.Split(domain, ".")
	for i := 1; i < len(parts)-1; i++ {
		parentDomain := strings.Join(parts[i:], ".")
		if _, ok := m.providers[parentDomain]; ok {
			return parentDomain, true
		}
		wildcardDomain := "*." + parentDomain
		if _, ok := m.providers[wildcardDomain]; ok {
			return wildcardDomain, true
		}
	}

	return "", false
}

func (m *ProviderMux) Present(domain, token, keyAuth string) error {
	provider, err := m.findProvider(domain)
	if err != nil {
		m.logger.Error().Err(err).Str("domain", domain).Msg("failed to find provider for domain")
		return err
	}

	m.logger.Debug().
		Str("domain", domain).
		Str("token", token[:8]+"...").
		Msg("presenting DNS challenge")

	return provider.Present(domain, token, keyAuth)
}

func (m *ProviderMux) CleanUp(domain, token, keyAuth string) error {
	provider, err := m.findProvider(domain)
	if err != nil {
		m.logger.Error().Err(err).Str("domain", domain).Msg("failed to find provider for domain")
		return err
	}

	m.logger.Debug().
		Str("domain", domain).
		Str("token", token[:8]+"...").
		Msg("cleaning up DNS challenge")

	return provider.CleanUp(domain, token, keyAuth)
}

func (m *ProviderMux) findProvider(domain string) (challenge.Provider, error) {
	challengeDomain := domain
	prefix := "_acme-challenge."
	if strings.HasPrefix(domain, prefix) {
		challengeDomain = domain[len(prefix):]
	}

	if provider, ok := m.providers[challengeDomain]; ok {
		return provider, nil
	}

	wildcardDomain := "*." + challengeDomain
	if provider, ok := m.providers[wildcardDomain]; ok {
		return provider, nil
	}

	parts := strings.Split(challengeDomain, ".")
	for i := 1; i < len(parts); i++ {
		parentDomain := strings.Join(parts[i:], ".")
		if provider, ok := m.providers[parentDomain]; ok {
			return provider, nil
		}

		wildcardParent := "*." + parentDomain
		if provider, ok := m.providers[wildcardParent]; ok {
			return provider, nil
		}
	}

	return nil, fmt.Errorf("no DNS provider found for domain: %s", domain)
}

type CertificateManager struct {
	user           *User
	client         *lego.Client
	providerMux    *ProviderMux
	certificates   *lru.Cache[string, *tls.Certificate]
	certMutex      sync.Mutex
	certDir        string
	logger         zerolog.Logger
	providerConfig *ProviderConfig
}

func NewCertificateManager(email, certDir, configPath string) (*CertificateManager, error) {
	const lruCacheSize = 512
	logger := shared.GetLogger("certs.manager")

	config, err := loadProviderConfig(configPath)
	if err != nil {
		return nil, fmt.Errorf("failed to load provider config: %w", err)
	}

	providerMux, err := NewProviderMux(config)
	if err != nil {
		return nil, fmt.Errorf("failed to create provider multiplexer: %w", err)
	}

	if err := os.MkdirAll(certDir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create certificate directory: %w", err)
	}

	certCache, err := lru.New[string, *tls.Certificate](lruCacheSize)
	if err != nil {
		return nil, fmt.Errorf("failed to create lru cache: %w", err)
	}

	keyPath := filepath.Join(certDir, "account.key")
	var privateKey crypto.PrivateKey
	keyBytes, err := os.ReadFile(keyPath)
	if err != nil {
		if os.IsNotExist(err) {
			logger.Info().Str("path", keyPath).Msg("no user private key found, generating a new one")
			newPrivateKey, err := ecdsa.GenerateKey(elliptic.P256(), rand.Reader)
			if err != nil {
				return nil, fmt.Errorf("failed to generate private key: %w", err)
			}
			privateKey = newPrivateKey

			pemKeyBytes, err := x509.MarshalECPrivateKey(newPrivateKey)
			if err != nil {
				return nil, fmt.Errorf("failed to marshal private key: %w", err)
			}
			pemKey := pem.EncodeToMemory(&pem.Block{Type: "EC PRIVATE KEY", Bytes: pemKeyBytes})
			if err := os.WriteFile(keyPath, pemKey, 0600); err != nil {
				return nil, fmt.Errorf("failed to save private key: %w", err)
			}
		} else {
			return nil, fmt.Errorf("failed to read private key: %w", err)
		}
	} else {
		logger.Info().Str("path", keyPath).Msg("loading existing user private key")
		block, _ := pem.Decode(keyBytes)
		if block == nil {
			return nil, fmt.Errorf("failed to decode pem block from private key")
		}
		p, err := x509.ParseECPrivateKey(block.Bytes)
		if err != nil {
			return nil, fmt.Errorf("failed to parse private key: %w", err)
		}
		privateKey = p
	}

	acmeUser := &User{Email: email, key: privateKey}

	regPath := filepath.Join(certDir, "account.json")
	regBytes, err := os.ReadFile(regPath)
	if err != nil {
		if os.IsNotExist(err) {
			logger.Info().Msg("no registration found, creating new one")
			tempConfig := lego.NewConfig(acmeUser)
			tempConfig.CADirURL = lego.LEDirectoryProduction
			tempConfig.Certificate.KeyType = certcrypto.EC256
			tempClient, err := lego.NewClient(tempConfig)
			if err != nil {
				return nil, fmt.Errorf("failed to create temp lego client: %w", err)
			}

			reg, err := tempClient.Registration.Register(registration.RegisterOptions{TermsOfServiceAgreed: true})
			if err != nil {
				return nil, fmt.Errorf("failed to register user: %w", err)
			}
			acmeUser.Registration = reg

			regJSON, err := json.MarshalIndent(reg, "", "  ")
			if err != nil {
				return nil, fmt.Errorf("failed to marshal registration: %w", err)
			}
			if err := os.WriteFile(regPath, regJSON, 0600); err != nil {
				return nil, fmt.Errorf("failed to save registration: %w", err)
			}
			logger.Info().Str("uri", reg.URI).Msg("new user registered")
		} else {
			return nil, fmt.Errorf("failed to read registration file: %w", err)
		}
	} else {
		var reg registration.Resource
		if err := json.Unmarshal(regBytes, &reg); err != nil {
			return nil, fmt.Errorf("failed to unmarshal registration file: %w", err)
		}
		acmeUser.Registration = &reg
		logger.Info().Str("uri", reg.URI).Msg("user registration loaded from disk")
	}

	finalConfig := lego.NewConfig(acmeUser)
	finalConfig.Certificate.KeyType = certcrypto.EC256
	finalConfig.CADirURL = lego.LEDirectoryProduction

	client, err := lego.NewClient(finalConfig)
	if err != nil {
		return nil, fmt.Errorf("failed to create lego client: %w", err)
	}

	err = client.Challenge.SetDNS01Provider(providerMux)
	if err != nil {
		return nil, fmt.Errorf("failed to set dns-01 provider: %w", err)
	}

	cm := &CertificateManager{
		user:           acmeUser,
		client:         client,
		providerMux:    providerMux,
		certificates:   certCache,
		certDir:        certDir,
		logger:         logger,
		providerConfig: config,
	}

	logger.Info().
		Str("email", email).
		Str("cert_dir", certDir).
		Str("ca_url", finalConfig.CADirURL).
		Msg("certificate manager initialized")

	return cm, nil
}

func (cm *CertificateManager) GetCertificate(hello *tls.ClientHelloInfo) (*tls.Certificate, error) {
	domain := strings.ToLower(hello.ServerName)
	if domain == "" {
		cm.logger.Warn().Msg("missing server name in tls handshake (sni), aborting handshake")
		return nil, nil
	}

	baseDomain, ok := cm.providerMux.FindManagedDomain(domain)
	if !ok {
		return nil, fmt.Errorf("domain %q is not configured for management", domain)
	}
	cm.logger.Debug().Str("domain", domain).Str("base_domain", baseDomain).Msg("certificate requested")

	cert, ok := cm.certificates.Get(baseDomain)
	if ok && cm.isCertificateValid(cert) {
		cm.logger.Debug().Str("domain", domain).Msg("using cached certificate (in-memory)")
		return cert, nil
	}

	cert, err := cm.loadCertificateFromDisk(baseDomain)
	if err == nil && cm.isCertificateValid(cert) {
		cm.logger.Info().Str("base_domain", baseDomain).Msg("loaded valid certificate from disk")
		cm.certificates.Add(baseDomain, cert)
		return cert, nil
	}

	cm.logger.Info().Str("domain", domain).Msg("no valid certificate found in cache or on disk, obtaining new one")
	return cm.obtainCertificate(baseDomain)
}

func (cm *CertificateManager) obtainCertificate(baseDomain string) (*tls.Certificate, error) {
	cm.certMutex.Lock()
	defer cm.certMutex.Unlock()

	if cert, ok := cm.certificates.Get(baseDomain); ok && cm.isCertificateValid(cert) {
		cm.logger.Debug().Str("domain", baseDomain).Msg("certificate was obtained by another goroutine while waiting for lock")
		return cert, nil
	}

	var domains []string
	if strings.HasPrefix(baseDomain, "*.") {
		domains = []string{baseDomain, strings.TrimPrefix(baseDomain, "*.")}
	} else {
		domains = []string{baseDomain}
	}

	cm.logger.Info().Strs("domains", domains).Msg("requesting new certificate from let's encrypt")

	request := certificate.ObtainRequest{
		Domains: domains,
		Bundle:  true,
	}

	resource, err := cm.client.Certificate.Obtain(request)
	if err != nil {
		return nil, fmt.Errorf("failed to obtain certificate: %w", err)
	}

	cm.saveCertificateToDisk(baseDomain, resource)

	tlsCert, err := tls.X509KeyPair(resource.Certificate, resource.PrivateKey)
	if err != nil {
		return nil, fmt.Errorf("failed to create tls key pair from obtained certificate: %w", err)
	}

	if len(tlsCert.Certificate) > 0 {
		x509Cert, err := x509.ParseCertificate(tlsCert.Certificate[0])
		if err != nil {
			return nil, fmt.Errorf("failed to parse obtained certificate leaf: %w", err)
		}
		tlsCert.Leaf = x509Cert
	}

	cm.certificates.Add(baseDomain, &tlsCert)

	cm.logger.Info().Str("domain", baseDomain).Time("expiry", tlsCert.Leaf.NotAfter).Msg("successfully obtained and cached certificate")
	return &tlsCert, nil
}

func (cm *CertificateManager) saveCertificateToDisk(baseDomain string, resource *certificate.Resource) {
	certPath := filepath.Join(cm.certDir, baseDomain+".crt")
	keyPath := filepath.Join(cm.certDir, baseDomain+".key")

	err := os.WriteFile(certPath, resource.Certificate, 0644)
	if err != nil {
		cm.logger.Error().Err(err).Str("path", certPath).Msg("failed to save certificate to disk")
	}

	err = os.WriteFile(keyPath, resource.PrivateKey, 0600)
	if err != nil {
		cm.logger.Error().Err(err).Str("path", keyPath).Msg("failed to save private key to disk")
	}

	cm.logger.Info().
		Str("domain", baseDomain).
		Str("cert_path", certPath).
		Str("key_path", keyPath).
		Msg("certificate saved to disk")
}

func (cm *CertificateManager) isCertificateValid(cert *tls.Certificate) bool {
	if len(cert.Certificate) == 0 {
		return false
	}

	x509Cert, err := x509.ParseCertificate(cert.Certificate[0])
	if err != nil {
		cm.logger.Error().Err(err).Msg("failed to parse certificate")
		return false
	}

	expirationThreshold := time.Now().Add(30 * 24 * time.Hour)

	if time.Now().After(x509Cert.NotAfter) {
		cm.logger.Debug().Time("expiry", x509Cert.NotAfter).Msg("certificate is expired")
		return false
	}

	if x509Cert.NotAfter.Before(expirationThreshold) {
		cm.logger.Debug().Time("expiry", x509Cert.NotAfter).Msg("certificate expires soon")
		return false
	}

	return true
}

func (cm *CertificateManager) PreloadCertificates() error {
	cm.logger.Info().Msg("starting certificate preload")

	managedDomains := make(map[string]struct{})
	for _, p := range cm.providerConfig.Providers {
		for _, d := range p.Domains {
			managedDomains[strings.ToLower(strings.TrimSpace(d))] = struct{}{}
		}
	}

	for domain := range managedDomains {
		cm.logger.Info().Str("domain", domain).Msg("checking certificate during preload")

		cert, ok := cm.certificates.Get(domain)
		if ok && cm.isCertificateValid(cert) {
			cm.logger.Info().Str("domain", domain).Msg("certificate is already in cache and valid")
			continue
		}

		cert, err := cm.loadCertificateFromDisk(domain)
		if err == nil && cm.isCertificateValid(cert) {
			cm.logger.Info().Str("domain", domain).Msg("loaded valid certificate from disk into cache")
			cm.certificates.Add(domain, cert)
			continue
		}

		cm.logger.Info().Str("domain", domain).Msg("no valid certificate on disk, obtaining new one")
		_, err = cm.obtainCertificate(domain)
		if err != nil {
			cm.logger.Error().Err(err).Str("domain", domain).Msg("failed to obtain certificate during preload")
			return fmt.Errorf("failed to obtain certificate for %s: %w", domain, err)
		} else {
			cm.logger.Info().Str("domain", domain).Msg("successfully obtained and cached certificate")
		}
	}

	cm.logger.Info().Msg("certificate preload finished successfully")
	return nil
}

func loadProviderConfig(configPath string) (*ProviderConfig, error) {
	logger := shared.GetLogger("certs.config")
	data, err := os.ReadFile(configPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read config file: %w", err)
	}

	var config ProviderConfig
	if err := json.Unmarshal(data, &config); err != nil {
		return nil, fmt.Errorf("failed to parse config JSON: %w", err)
	}

	logger.Info().Str("path", configPath).Int("providers", len(config.Providers)).Msg("loaded DNS provider config")
	return &config, nil
}

func (cm *CertificateManager) loadCertificateFromDisk(baseDomain string) (*tls.Certificate, error) {
	certPath := filepath.Join(cm.certDir, baseDomain+".crt")
	keyPath := filepath.Join(cm.certDir, baseDomain+".key")

	certBytes, err := os.ReadFile(certPath)
	if err != nil {
		return nil, err
	}

	keyBytes, err := os.ReadFile(keyPath)
	if err != nil {
		return nil, err
	}

	tlsCert, err := tls.X509KeyPair(certBytes, keyBytes)
	if err != nil {
		return nil, err
	}

	if len(tlsCert.Certificate) > 0 {
		x509Cert, err := x509.ParseCertificate(tlsCert.Certificate[0])
		if err != nil {
			return nil, fmt.Errorf("failed to parse certificate to get leaf: %w", err)
		}
		tlsCert.Leaf = x509Cert
	}

	return &tlsCert, nil
}
