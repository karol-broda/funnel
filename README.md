![funnel homepage flow-field graphic](docs/public/readme-hero.png)

a self-hosted tunneling server and client. run the server on your own infrastructure, point the client at it, and expose local services through secure QUIC tunnels.

## server

```bash
funnel-server --port 8080 --quic-port 4433
```

by default the server uses an embedded turso/libsql database at `./funnel.db`. pass `--database-url` to use postgres instead. migrations run on startup.

every cli flag has a corresponding environment variable. a docker image is also available.

### auth

bootstrap with a seed key:

```bash
funnel-server --seed-api-key
```

prints an api key to stdout. use it to create additional keys or configure oauth.

github oauth:

```bash
funnel-server \
  --base-url https://tunnel.company.com \
  --github-client-id $GITHUB_CLIENT_ID \
  --github-client-secret $GITHUB_CLIENT_SECRET
```

any OIDC-compatible provider works through the generic oauth flags (`--oauth-provider-name`, `--oauth-client-id`, etc). both can run simultaneously.

the first user to log in is promoted to admin. `--initial-admin-email` controls subsequent auto-promotion.

api keys have scopes: `tunnels` for tunnel connections, `management` for api access.

### tls

```bash
funnel-server \
  --enable-tls \
  --letsencrypt-email you@example.com \
  --dns-providers-config providers.json
```

certificates are obtained through let's encrypt DNS-01 challenges and auto-renew 30 days before expiry. dns providers: cloudflare, route53, or an exec provider that delegates to a script.

## client

```bash
curl -sSfL https://raw.githubusercontent.com/karol-broda/funnel/master/scripts/install.sh | bash
```

linux and macos, amd64 and arm64. the script verifies checksums before installing.

```bash
cargo install --path crates/funnel-client
```

### tunnel

```bash
funnel http 3000
```

connects to the server, registers a tunnel, and forwards requests to `localhost:3000`. the public url is printed on connect.

```bash
funnel http 3000 --id my-app
funnel http 3000 --id staging --team backend
```

`--id` sets the subdomain. `--team` associates the tunnel with a team.

each http request runs on its own QUIC stream. websocket upgrades are forwarded as bidirectional byte streams.

### contexts

the client supports named contexts for connecting to multiple servers.

```bash
funnel context create work --server https://tunnel.company.com
funnel context use work
funnel login
```

tokens are stored in `~/.config/funnel/config.toml` with `0600` permissions. `FUNNEL_SERVER` and `FUNNEL_TOKEN` override the active context.

### management

```bash
funnel status # active tunnels
funnel whoami  # current user
funnel keys create my-key --scopes tunnels # create api key
funnel keys list # list api keys
funnel sessions # tunnel session history
funnel users list # all users (admin)
funnel users set-role <id> admin # set role (admin)
funnel teams create backend # create team (admin)
funnel teams add-member <team> <user> # add team member
```

## nix

the flake provides packages, OCI containers, a NixOS module, and a Home Manager module. a binary cache is available so you don't have to build from source.

### binary cache

```nix
# flake.nix (or accept the nixConfig prompt when first using the flake)
nix.settings = {
  substituters = ["https://cache.karolbroda.com/funnel"];
  trusted-public-keys = ["funnel:f2v8GXhe4t/c5ITHJ/LRYqcen5RWsYNEa9BKvxxxVBA="];
};
```

### packages

```bash
nix build .#funnel-server
nix build .#funnel-client
nix run .#funnel-client -- http 3000
```

### OCI containers

```bash
nix build .#funnel-server-image
docker load < result
docker run -p 8080:8080 -p 4433:4433/udp funnel-server:latest
```

the images are built from nix, not from a Dockerfile. they contain only the binary, ca certificates, and timezone data. no shell, no package manager. set `DATABASE_URL` at runtime for postgres, otherwise the embedded turso database is used.

### NixOS module

```nix
{
  inputs.funnel.url = "github:karol-broda/funnel";

  # configuration.nix
  imports = [inputs.funnel.nixosModules.funnel-server];

  services.funnel.server = {
    enable = true;
    port = 8080;
    quicPort = 4433;
    openFirewall = true;

    # secrets via environment file (agenix, sops-nix, or plain file)
    environmentFile = config.age.secrets.funnel.path;

    tls = {
      enable = true;
      port = 443;
      acme.email = "admin@example.com";
      dnsProvidersConfigFile = ./dns-providers.json;
    };

    auth = {
      baseUrl = "https://tunnel.example.com";
      github.clientId = "Iv1.abc123";
      initialAdminEmail = "admin@example.com";
    };

    # auto-provision a local postgres database
    database.createLocally = true;
  };
}
```

the systemd service runs with `DynamicUser`, a private `/tmp`, restricted syscalls, and the full set of hardening options. secrets go in an `environmentFile` rather than the nix store.

### Home Manager module

```nix
{
  inputs.funnel.url = "github:karol-broda/funnel";

  imports = [inputs.funnel.homeManagerModules.funnel-client];

  programs.funnel = {
    enable = true;
    settings = {
      currentContext = "production";
      contexts = {
        production.server = "https://tunnel.example.com";
        staging.server = "https://tunnel.staging.example.com";
      };
    };
  };
}
```

when no `tokenFile` is used, the config is a read-only symlink to the nix store. when any context sets `tokenFile`, the config is generated at activation time as a regular file with `0600` permissions, so tokens never enter the store.

you can also skip declarative token management entirely and set `FUNNEL_TOKEN` in your shell environment, or leave contexts empty and use `funnel login`.

with sops-nix:

```nix
sops.secrets."funnel/production-token" = {};
sops.secrets."funnel/staging-token" = {};

programs.funnel = {
  enable = true;
  settings.contexts = {
    production = {
      server = "https://tunnel.example.com";
      tokenFile = config.sops.secrets."funnel/production-token".path;
    };
    staging = {
      server = "https://tunnel.staging.example.com";
      tokenFile = config.sops.secrets."funnel/staging-token".path;
    };
  };
};
```

### overlay

```nix
nixpkgs.overlays = [inputs.funnel.overlays.default];
environment.systemPackages = [pkgs.funnel-client];
```

## development

```bash
nix develop
just build
just test
just e2e
```

e2e tests start a real server and client, create a tunnel, and send http requests, websocket messages, and large payloads through it.

NixOS VM tests run a full server in a QEMU VM and verify the health endpoint, authentication, systemd hardening, restart behavior, and client connectivity:

```bash
nix build .#checks.x86_64-linux.nixos-server
```

### nix recipes

```bash
just nix-build # build server + client
just nix-images # build OCI container images
just nix-check # run clippy, fmt, and VM tests
just nix-push # push packages to the attic binary cache
just nix-push-images # push container images to the cache
just nix-docker-load # load a container image into docker
```

## license

[mit](./LICENSE.md)
