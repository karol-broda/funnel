# tls setup guide

this guide explains how to configure and use the tunnel server with automatic let's encrypt tls certificates using multiple dns providers.

## overview

the tunnel server now supports automatic tls certificate management using let's encrypt with dns-01 challenges. this allows you to:

- generate wildcard certificates (`*.example.com`)
- support multiple domains across different dns providers
- handle multiple aws accounts/tenants
- automatically renew certificates before expiration
- use sni (server name indication) for multi-domain support

## prerequisites

1. **domain control**: you must control the dns for the domains you want certificates for
2. **dns provider access**: api credentials for your dns provider(s)
3. **email address**: required for let's encrypt registration

## supported dns providers

the server dynamically supports **any dns provider** available in the [go-acme/lego](https://go-acme.github.io/lego/dns/index.html) library. you are no longer limited to a hardcoded list.

to use a provider, you must specify its name in the `provider` field and provide the required credentials in the `config` map. the keys in the `config` map **must match the environment variable names** that the specific lego provider expects.

for a complete list of providers and their required environment variables, see the official lego dns provider documentation: [https://go-acme.github.io/lego/dns/index.html](https://go-acme.github.io/lego/dns/index.html)

for example, to use `gandiv5`, you would find its required environment variable (`gandiv5_api_key`) and use that as the key in your config.

### 1. dns providers configuration

create a `dns-providers.json` file. the structure of this file is defined by the `dns-providers.schema.json` schema.

here is a basic example using route53 and cloudflare:

```json
{
  "providers": [
    {
      "name": "aws-production",
      "provider": "route53",
      "domains": ["example.com", "*.example.com"],
      "config": {
        "aws_access_key_id": "your-access-key",
        "aws_secret_access_key": "your-secret-key",
        "aws_region": "us-east-1"
      }
    },
    {
      "name": "cloudflare-main",
      "provider": "cloudflare",
      "domains": ["myapp.org", "*.myapp.org"],
      "config": {
        "cloudflare_dns_api_token": "your-api-token"
      }
    }
  ]
}
```

### 2. route53 configuration options

for aws route53, you can configure:

```json
{
  "name": "aws-tenant",
  "provider": "route53",
  "domains": ["example.com", "*.example.com"],
  "config": {
    "aws_access_key_id": "akiaiosfodnn7example",
    "aws_secret_access_key": "wjalrxutnfemi/k7mdeng/bpxrficyexamplekey",
    "aws_region": "us-east-1",
    "aws_session_token": "optional-session-token",
    "aws_assume_role_arn": "arn:aws:iam::123456789012:role/route53access"
  }
}
```

**note**: `aws_profile` is handled automatically by the aws sdk through environment variables and is not needed in the config.

### 3. cloudflare configuration options

for cloudflare, you can use either:

**api token (recommended)**:
```json
{
  "name": "cloudflare-token",
  "provider": "cloudflare",
  "domains": ["example.com", "*.example.com"],
  "config": {
    "cloudflare_dns_api_token": "your-scoped-api-token"
  }
}
```

**global api key**:
```json
{
  "name": "cloudflare-global",
  "provider": "cloudflare",
  "domains": ["example.com", "*.example.com"],
  "config": {
    "cloudflare_email": "your-email@example.com",
    "cloudflare_api_key": "your-global-api-key"
  }
}
```

### example: digitalocean

to configure digitalocean, you would find its documentation requires `do_auth_token`. your configuration would look like this:

```json
{
  "name": "digitalocean-main",
  "provider": "digitalocean",
  "domains": ["do-domain.com", "*.do-domain.com"],
  "config": {
    "do_auth_token": "your-digitalocean-api-token"
  }
}
```

## running the server

### basic tls setup

```bash
./tunnel-server \
  --enable-tls \
  --email your-email@example.com \
  --dns-config ./dns-providers.json \
  --cert-dir ./certs \
  --tls-port 443 \
  --port 80
```

### development/testing

for development, you might want to use different ports:

```bash
./tunnel-server \
  --enable-tls \
  --email your-email@example.com \
  --dns-config ./dns-providers.json \
  --tls-port 8443 \
  --port 8080
```

### production setup

for production, typically you'd run on standard ports:

```bash
sudo ./tunnel-server \
  --enable-tls \
  --email your-email@example.com \
  --dns-config /etc/funnel/dns-providers.json \
  --cert-dir /var/lib/funnel/certs \
  --tls-port 443 \
  --port 80
```

## how it works

### certificate management

1. **on-demand generation**: certificates are generated automatically when a client connects with sni
2. **wildcard support**: the system automatically requests both the specific domain and wildcard (`*.domain.com`)
3. **caching**: certificates are cached in memory and persisted to disk
4. **auto-renewal**: certificates are automatically renewed when they expire within 30 days

### dns challenge process

1. client connects with sni for `subdomain.example.com`
2. system checks if a valid certificate exists
3. if not, it determines the appropriate dns provider based on domain mapping
4. creates dns txt record `_acme-challenge.subdomain.example.com`
5. let's encrypt validates the challenge
6. certificate is issued and cached
7. dns txt record is cleaned up

### multi-provider routing

the system uses a "provider multiplexer" that routes dns challenges to the correct provider:

- exact domain match: `example.com` → provider for `example.com`
- wildcard match: `sub.example.com` → provider for `*.example.com`
- parent domain match: `deep.sub.example.com` → provider for `example.com`

## security considerations

1. **credential storage**: dns provider credentials are stored in the configuration file - ensure proper file permissions
2. **certificate storage**: certificates are stored in the specified cert directory with appropriate permissions
3. **tls configuration**: the server uses secure tls settings with modern cipher suites
4. **rate limits**: be aware of let's encrypt rate limits (50 certificates per registered domain per week)

## troubleshooting

### common issues

1. **dns propagation**: dns changes can take time to propagate. the system waits for propagation automatically.
2. **rate limits**: if you hit let's encrypt rate limits, you'll need to wait or use the staging environment for testing.
3. **dns provider errors**: check your dns provider credentials and permissions.

### logging

the system provides detailed logging for certificate operations:

```bash
# enable debug logging
export log_level=debug
./tunnel-server --enable-tls ...
```

### testing with let's encrypt staging

for testing, you can modify the code to use let's encrypt staging environment by changing:

```go
legoconfig.cadirurl = lego.ledirectorystaging
```

## file structure

```
./
├── tunnel-server              # main executable
├── dns-providers.json         # dns providers configuration
├── dns-providers.schema.json  # example configuration
├── certs/                     # certificate storage directory
│   ├── example.com.crt       # certificate files
│   ├── example.com.key       # private key files
│   └── ...
└── tls_setup.md              # this guide
```

## example systemd service

```ini
[unit]
description=tunnel server with tls
after=network.target

[service]
type=simple
user=tunnel
workingdirectory=/opt/tunnel-server
execstart=/opt/tunnel-server/tunnel-server \
  --enable-tls \
  --email admin@example.com \
  --dns-config /opt/tunnel-server/dns-providers.json \
  --cert-dir /var/lib/tunnel-server/certs \
  --tls-port 443 \
  --port 80
restart=always
restartsec=5

[install]
wantedby=multi-user.target
```