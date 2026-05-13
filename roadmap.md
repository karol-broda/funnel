# roadmap

## done

- HTTP tunneling over QUIC with per-request streams
- WebSocket forwarding
- custom tunnel IDs (subdomains)
- auto-reconnection with exponential backoff
- API key auth with scopes (`tunnels`, `management`)
- OAuth login (GitHub + generic OIDC)
- teams, roles, user management
- session tracking and bandwidth metrics
- TLS with automatic Let's Encrypt (DNS-01 via Cloudflare, Route53, exec provider)
- Prometheus metrics
- proxy headers (`X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Proto`)
- dual database backend (PostgreSQL, embedded Turso)
- REST API with OpenAPI spec
- NixOS module with systemd hardening
- Home Manager module with sops-nix support
- OCI container images
- multi-arch releases (linux/macOS, amd64/arm64)
- install script with checksum verification

## project config file

a `funnel.toml` in the project root that declares tunnels. `funnel up` reads it and starts everything.

```toml
[tunnels.api]
port = 3000
id = "my-api"
team = "backend"

[tunnels.frontend]
port = 5173
id = "my-frontend"
```

committable to the repo so anyone on the team gets the same tunnel setup with one command.

## multiple tunnels per client

run multiple tunnels from a single process, configured via `funnel.toml` or CLI args. both approaches share a single QUIC connection to the server.

```bash
funnel http 3000 --id api -- http 5173 --id frontend
```

## shell completions

```bash
funnel completion bash > /etc/bash_completion.d/funnel
funnel completion zsh > ~/.zfunc/_funnel
funnel completion fish > ~/.config/fish/completions/funnel.fish
```

also generated in the Home Manager module.

## self-update

```bash
funnel update
funnel update --check
```

checks GitHub releases. prints a notice on `funnel http` when a new version is available, suppressible via config or `FUNNEL_NO_UPDATE_CHECK`. no silent auto-updates.

## request inspection and replay

a local web UI for inspecting requests flowing through your tunnels.

```
tunnel active: https://my-api.tunnel.example.com → localhost:3000
inspector:     http://127.0.0.1:4040
```

live request/response feed with headers, body, timing. filter by status code, method, path. replay requests. WebSocket frame inspection. export as curl commands.

runs as an embedded HTTP server in the client process.

## file serving

```bash
funnel file ./dist
funnel file ./dist --id docs
funnel file ./report.html
```

embedded static file server with directory listing, content-type detection, and range requests.

## tunnel access control

protect individual tunnels without changing the application behind them.

```bash
funnel http 3000 --auth user:pass
funnel http 3000 --allow-ip 10.0.0.0/8
funnel http 3000 --expires 2h
```

```toml
[tunnels.api.access]
auth = "user:pass"
allow_ips = ["10.0.0.0/8", "192.168.0.0/16"]
expires = "4h"
```

also: OAuth-gated access (require login to reach the tunnel) and team-scoped access (only members of the tunnel's team can reach it).

## rate limiting

per-user, per-team, and per-tunnel rate limits enforced at the server. connection rate, request rate, and bandwidth limits. configurable server-side.

## audit logging

structured log of security-relevant events: tunnel created/destroyed, auth attempts, access control denials, API key operations, team membership changes. emitted as JSON, forwardable to any log aggregator.

## OpenTelemetry

distributed traces through the tunnel (client → server → client, per-request spans), OTLP metric export alongside existing Prometheus, and structured log export via OTLP.

configurable via standard `OTEL_EXPORTER_OTLP_ENDPOINT` environment variables. the Prometheus endpoint stays.

## web dashboard

server-side web UI. active tunnel overview with live status, per-tunnel and per-team usage graphs, user and team management, API key management, session history. the Leptos crate already exists as a stub.

## HTTP/2 proxying

the tunnel currently proxies HTTP/1.1. HTTP/2 support enables gRPC tunneling and multiplexed browser requests.

## TCP tunneling

```bash
funnel tcp 5432 --id my-db
funnel tcp 22 --id my-ssh
```

raw TCP forwarding without HTTP framing. the server allocates a port and forwards bytes bidirectionally.

## TLS passthrough

```bash
funnel tls 8443 --id secure-app
```

forward TLS connections without terminating them at the server. routes via SNI. useful for applications that handle their own TLS or need mutual TLS.

## UDP tunneling

```bash
funnel udp 51820 --id wireguard
funnel udp 27015 --id game-server
```

QUIC already runs over UDP so the transport is a natural fit. lowest priority of the protocol additions.
