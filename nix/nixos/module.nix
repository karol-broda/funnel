self: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.services.funnel.server;

  needsPrivilegedPorts =
    cfg.port < 1024
    || cfg.tls.port < 1024
    || cfg.quicPort < 1024;
in {
  options.services.funnel.server = {
    enable = lib.mkEnableOption "funnel tunnel server";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.funnel-server;
      description = "The funnel-server package to use.";
    };

    host = lib.mkOption {
      type = lib.types.str;
      default = "0.0.0.0";
      description = "Address to bind the HTTP server to.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
      description = "HTTP port for the API and proxy.";
    };

    quicPort = lib.mkOption {
      type = lib.types.port;
      default = 4433;
      description = "UDP port for QUIC tunnel connections.";
    };

    openFirewall = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Open firewall ports for HTTP, TLS, and QUIC.";
    };

    seedApiKey = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Create a seed API key on startup (printed to journal).";
    };

    logLevel = lib.mkOption {
      type = lib.types.str;
      default = "info";
      description = ''
        Log level filter (RUST_LOG syntax).
        Examples: "info", "debug", "funnel_server=debug,tower_http=trace"
      '';
    };

    environmentFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = ''
        Environment file for secrets (DATABASE_URL, GITHUB_CLIENT_SECRET, etc.).
        Compatible with agenix, sops-nix, or plain files.
      '';
    };

    environmentFiles = lib.mkOption {
      type = lib.types.listOf lib.types.path;
      default = [];
      description = ''
        List of environment files loaded by systemd.
        Use this when secrets come from multiple sources.
      '';
    };

    database = {
      postgresUrl = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = ''
          PostgreSQL connection URL. When null, the embedded Turso database is used.
          For secrets, set DATABASE_URL via environmentFile instead.
        '';
      };

      maxConnections = lib.mkOption {
        type = lib.types.ints.positive;
        default = 10;
        description = "Maximum PostgreSQL connection pool size.";
      };

      tursoPath = lib.mkOption {
        type = lib.types.str;
        default = "/var/lib/funnel/funnel.db";
        description = "Path to the embedded Turso/libsql database file.";
      };

      createLocally = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = ''
          Whether to create a local PostgreSQL database.
          When enabled, a database named 'funnel' is created automatically
          and the server connects to it via unix socket.
        '';
      };
    };

    tls = {
      enable = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = "Enable TLS with automatic Let's Encrypt certificates.";
      };

      port = lib.mkOption {
        type = lib.types.port;
        default = 8443;
        description = "HTTPS port.";
      };

      certDir = lib.mkOption {
        type = lib.types.str;
        default = "/var/lib/funnel/certs";
        description = "Directory for TLS certificate storage.";
      };

      acme = {
        email = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Email for ACME registration. Required when TLS is enabled.";
        };

        staging = lib.mkOption {
          type = lib.types.bool;
          default = false;
          description = "Use Let's Encrypt staging environment.";
        };
      };

      dnsProvidersConfigFile = lib.mkOption {
        type = lib.types.nullOr lib.types.path;
        default = null;
        description = "Path to DNS providers JSON config. Required when TLS is enabled.";
      };
    };

    auth = {
      baseUrl = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Public base URL. Required when any OAuth provider is configured.";
      };

      initialAdminEmail = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        default = null;
        description = "Email auto-promoted to admin on first login.";
      };

      github = {
        clientId = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "GitHub OAuth client ID.";
        };
      };

      oauth = {
        providerName = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Generic OAuth provider name (e.g. 'gitlab', 'google').";
        };

        clientId = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "Generic OAuth client ID.";
        };

        authorizeUrl = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "OAuth authorization endpoint.";
        };

        tokenUrl = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "OAuth token exchange endpoint.";
        };

        userinfoUrl = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
          description = "OAuth userinfo endpoint.";
        };

        scopes = lib.mkOption {
          type = lib.types.str;
          default = "openid email profile";
          description = "OAuth scopes (space-separated).";
        };

        idField = lib.mkOption {
          type = lib.types.str;
          default = "sub";
          description = "Userinfo JSON field for user ID.";
        };

        emailField = lib.mkOption {
          type = lib.types.str;
          default = "email";
          description = "Userinfo JSON field for email.";
        };

        nameField = lib.mkOption {
          type = lib.types.str;
          default = "name";
          description = "Userinfo JSON field for display name.";
        };

        avatarField = lib.mkOption {
          type = lib.types.str;
          default = "picture";
          description = "Userinfo JSON field for avatar URL.";
        };
      };
    };

    extraEnvironment = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = {};
      description = "Additional environment variables for the server process.";
    };

    serviceConfig = lib.mkOption {
      type = lib.types.attrsOf lib.types.anything;
      default = {};
      description = ''
        Extra systemd service configuration merged into serviceConfig.
        Use this to override hardening settings or add resource limits.
        Example: { MemoryMax = "512M"; CPUQuota = "200%"; }
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.tls.enable -> cfg.tls.acme.email != null;
        message = "services.funnel.server.tls.acme.email is required when TLS is enabled.";
      }
      {
        assertion = cfg.tls.enable -> cfg.tls.dnsProvidersConfigFile != null;
        message = "services.funnel.server.tls.dnsProvidersConfigFile is required when TLS is enabled.";
      }
      {
        assertion =
          (cfg.auth.github.clientId != null || cfg.auth.oauth.providerName != null)
          -> cfg.auth.baseUrl != null;
        message = "services.funnel.server.auth.baseUrl is required when OAuth is configured.";
      }
      {
        assertion = cfg.database.createLocally -> cfg.database.postgresUrl == null;
        message = "services.funnel.server.database.postgresUrl must be null when createLocally is true.";
      }
    ];

    services.postgresql = lib.mkIf cfg.database.createLocally {
      enable = true;
      ensureDatabases = ["funnel"];
      ensureUsers = [
        {
          name = "funnel";
          ensureDBOwnership = true;
        }
      ];
    };

    systemd.services.funnel-server = {
      description = "Funnel tunnel server";
      after =
        ["network-online.target"]
        ++ lib.optionals (cfg.database.postgresUrl != null || cfg.database.createLocally) ["postgresql.service"];
      wants = ["network-online.target"];
      wantedBy = ["multi-user.target"];

      environment =
        {
          FUNNEL_HOST = cfg.host;
          FUNNEL_PORT = toString cfg.port;
          FUNNEL_QUIC_PORT = toString cfg.quicPort;
          FUNNEL_DB_MAX_CONNECTIONS = toString cfg.database.maxConnections;
          FUNNEL_TURSO_DB_PATH = cfg.database.tursoPath;
          FUNNEL_CERT_DIR = cfg.tls.certDir;
          FUNNEL_TLS_PORT = toString cfg.tls.port;
          RUST_LOG = cfg.logLevel;
          OAUTH_SCOPES = cfg.auth.oauth.scopes;
          OAUTH_ID_FIELD = cfg.auth.oauth.idField;
          OAUTH_EMAIL_FIELD = cfg.auth.oauth.emailField;
          OAUTH_NAME_FIELD = cfg.auth.oauth.nameField;
          OAUTH_AVATAR_FIELD = cfg.auth.oauth.avatarField;
        }
        // lib.optionalAttrs cfg.tls.enable {FUNNEL_ENABLE_TLS = "true";}
        // lib.optionalAttrs cfg.tls.acme.staging {FUNNEL_ACME_STAGING = "true";}
        // lib.optionalAttrs cfg.seedApiKey {FUNNEL_SEED_API_KEY = "true";}
        // lib.optionalAttrs (cfg.database.postgresUrl != null) {DATABASE_URL = cfg.database.postgresUrl;}
        // lib.optionalAttrs cfg.database.createLocally {DATABASE_URL = "postgres:///funnel?host=/run/postgresql";}
        // lib.optionalAttrs (cfg.tls.acme.email != null) {LETSENCRYPT_EMAIL = cfg.tls.acme.email;}
        // lib.optionalAttrs (cfg.tls.dnsProvidersConfigFile != null) {DNS_PROVIDERS_CONFIG = toString cfg.tls.dnsProvidersConfigFile;}
        // lib.optionalAttrs (cfg.auth.baseUrl != null) {BASE_URL = cfg.auth.baseUrl;}
        // lib.optionalAttrs (cfg.auth.initialAdminEmail != null) {FUNNEL_INITIAL_ADMIN_EMAIL = cfg.auth.initialAdminEmail;}
        // lib.optionalAttrs (cfg.auth.github.clientId != null) {GITHUB_CLIENT_ID = cfg.auth.github.clientId;}
        // lib.optionalAttrs (cfg.auth.oauth.providerName != null) {OAUTH_PROVIDER_NAME = cfg.auth.oauth.providerName;}
        // lib.optionalAttrs (cfg.auth.oauth.clientId != null) {OAUTH_CLIENT_ID = cfg.auth.oauth.clientId;}
        // lib.optionalAttrs (cfg.auth.oauth.authorizeUrl != null) {OAUTH_AUTHORIZE_URL = cfg.auth.oauth.authorizeUrl;}
        // lib.optionalAttrs (cfg.auth.oauth.tokenUrl != null) {OAUTH_TOKEN_URL = cfg.auth.oauth.tokenUrl;}
        // lib.optionalAttrs (cfg.auth.oauth.userinfoUrl != null) {OAUTH_USERINFO_URL = cfg.auth.oauth.userinfoUrl;}
        // cfg.extraEnvironment;

      serviceConfig =
        {
          ExecStart = lib.getExe cfg.package;
          Restart = "always";
          RestartSec = 5;

          DynamicUser = !cfg.database.createLocally;
          User = lib.mkIf cfg.database.createLocally "funnel";
          Group = lib.mkIf cfg.database.createLocally "funnel";
          StateDirectory = "funnel";
          StateDirectoryMode = "0750";

          NoNewPrivileges = true;
          ProtectSystem = "strict";
          ProtectHome = true;
          PrivateTmp = true;
          PrivateDevices = true;
          ProtectKernelTunables = true;
          ProtectKernelModules = true;
          ProtectControlGroups = true;
          RestrictSUIDSGID = true;
          RestrictNamespaces = true;
          LockPersonality = true;
          RestrictRealtime = true;
          SystemCallFilter = ["@system-service" "~@privileged"];

          EnvironmentFile =
            lib.optional (cfg.environmentFile != null) cfg.environmentFile
            ++ cfg.environmentFiles;
        }
        // lib.optionalAttrs needsPrivilegedPorts {
          CapabilityBoundingSet = ["CAP_NET_BIND_SERVICE"];
          AmbientCapabilities = ["CAP_NET_BIND_SERVICE"];
        }
        // cfg.serviceConfig;
    };

    users = lib.mkIf cfg.database.createLocally {
      users.funnel = {
        isSystemUser = true;
        group = "funnel";
      };
      groups.funnel = {};
    };

    networking.firewall = lib.mkIf cfg.openFirewall {
      allowedTCPPorts =
        [cfg.port]
        ++ lib.optionals cfg.tls.enable [cfg.tls.port];
      allowedUDPPorts = [cfg.quicPort];
    };
  };
}
