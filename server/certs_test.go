package server

import (
	"fmt"
	"testing"

	"github.com/go-acme/lego/v4/challenge"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

type mockProvider struct {
	name      string
	presentFn func(domain, token, keyAuth string) error
	cleanUpFn func(domain, token, keyAuth string) error
}

func (m *mockProvider) Present(domain, token, keyAuth string) error {
	if m.presentFn != nil {
		return m.presentFn(domain, token, keyAuth)
	}
	return nil
}

func (m *mockProvider) CleanUp(domain, token, keyAuth string) error {
	if m.cleanUpFn != nil {
		return m.cleanUpFn(domain, token, keyAuth)
	}
	return nil
}

func TestNewProviderMux(t *testing.T) {
	mockProviderCreator := func(pconf DNSProviderConfig) (challenge.Provider, error) {
		if pconf.Provider == "unsupported" {
			return nil, fmt.Errorf("unsupported DNS provider: %s", pconf.Provider)
		}
		return &mockProvider{name: pconf.Name}, nil
	}

	tests := []struct {
		name          string
		config        *ProviderConfig
		expectedError string
		validate      func(t *testing.T, mux *ProviderMux)
	}{
		{
			name: "valid config with multiple providers and domains",
			config: &ProviderConfig{
				Providers: []DNSProviderConfig{
					{
						Name:     "aws-prod",
						Provider: "route53",
						Domains:  []string{"example.com", "*.example.com"},
						Config:   map[string]string{"AWS_REGION": "us-east-1"},
					},
					{
						Name:     "cloudflare-main",
						Provider: "cloudflare",
						Domains:  []string{"myapp.org", "*.myapp.org"},
						Config:   map[string]string{"CLOUDFLARE_DNS_API_TOKEN": "secret"},
					},
				},
			},
			validate: func(t *testing.T, mux *ProviderMux) {
				require.NotNil(t, mux)
				assert.Equal(t, 4, len(mux.providers))

				p1 := mux.providers["example.com"]
				p2 := mux.providers["*.example.com"]
				p3 := mux.providers["myapp.org"]
				p4 := mux.providers["*.myapp.org"]

				require.NotNil(t, p1)
				require.NotNil(t, p2)
				require.NotNil(t, p3)
				require.NotNil(t, p4)

				assert.True(t, p1 == p2, "Expected same provider instance for example.com and *.example.com")
				assert.True(t, p3 == p4, "Expected same provider instance for myapp.org and *.myapp.org")
				assert.False(t, p1 == p3, "Expected different provider instances for different providers")

				assert.Equal(t, "aws-prod", p1.(*mockProvider).name)
				assert.Equal(t, "cloudflare-main", p3.(*mockProvider).name)
			},
		},
		{
			name: "unsupported provider",
			config: &ProviderConfig{
				Providers: []DNSProviderConfig{
					{
						Name:     "unsupported",
						Provider: "unsupported",
						Domains:  []string{"test.com"},
					},
				},
			},
			expectedError: "unsupported DNS provider: unsupported",
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mux, err := newProviderMuxWithCreator(tc.config, mockProviderCreator)

			if tc.expectedError != "" {
				require.Error(t, err)
				assert.Contains(t, err.Error(), tc.expectedError)
				assert.Nil(t, mux)
			} else {
				require.NoError(t, err)
				tc.validate(t, mux)
			}
		})
	}
}

func TestProviderMux_FindProvider(t *testing.T) {
	mockProviderCreator := func(pconf DNSProviderConfig) (challenge.Provider, error) {
		return &mockProvider{name: pconf.Name}, nil
	}

	config := &ProviderConfig{
		Providers: []DNSProviderConfig{
			{
				Name:     "aws-prod",
				Provider: "route53",
				Domains:  []string{"example.com", "*.example.com"},
			},
			{
				Name:     "cloudflare-main",
				Provider: "cloudflare",
				Domains:  []string{"myapp.org"},
			},
			{
				Name:     "aws-staging",
				Provider: "route53",
				Domains:  []string{"staging.example.com", "*.staging.example.com"},
			},
		},
	}

	mux, err := newProviderMuxWithCreator(config, mockProviderCreator)
	require.NoError(t, err)
	require.NotNil(t, mux)

	tests := []struct {
		domain           string
		expectedProvider string
		expectError      bool
	}{
		{domain: "example.com", expectedProvider: "aws-prod"},
		{domain: "www.example.com", expectedProvider: "aws-prod"},
		{domain: "test.example.com", expectedProvider: "aws-prod"},
		{domain: "_acme-challenge.example.com", expectedProvider: "aws-prod"},
		{domain: "_acme-challenge.www.example.com", expectedProvider: "aws-prod"},
		{domain: "myapp.org", expectedProvider: "cloudflare-main"},
		{domain: "staging.example.com", expectedProvider: "aws-staging"},
		{domain: "api.staging.example.com", expectedProvider: "aws-staging"},
		{domain: "unknown.com", expectError: true},
		{domain: "example.org", expectError: true},
	}

	for _, tc := range tests {
		t.Run(tc.domain, func(t *testing.T) {
			provider, err := mux.findProvider(tc.domain)
			if tc.expectError {
				require.Error(t, err)
				assert.Nil(t, provider)
			} else {
				require.NoError(t, err)
				require.NotNil(t, provider)
				assert.Equal(t, tc.expectedProvider, provider.(*mockProvider).name)
			}
		})
	}
}
