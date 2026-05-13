self: {
  config,
  lib,
  pkgs,
  ...
}: let
  cfg = config.programs.funnel;
  tomlFormat = pkgs.formats.toml {};

  hasContexts = cfg.settings.contexts != {};
  hasTokenFiles = lib.any (ctx: ctx.tokenFile != null) (lib.attrValues cfg.settings.contexts);

  staticConfigContents =
    {
      current_context = cfg.settings.currentContext;
    }
    // lib.optionalAttrs hasContexts {
      contexts = lib.mapAttrs (_: ctx:
        {server = ctx.server;}
        // lib.optionalAttrs (ctx.token != null) {token = ctx.token;})
      cfg.settings.contexts;
    };

  activationScript = let
    contextEntries = lib.concatStrings (lib.mapAttrsToList (name: ctx: ''
      printf '[contexts.${name}]\nserver = "%s"\n' '${ctx.server}' >> "$config_path"
      ${lib.optionalString (ctx.tokenFile != null) ''
        printf 'token = "%s"\n' "$(cat '${ctx.tokenFile}')" >> "$config_path"
      ''}
      ${lib.optionalString (ctx.token != null && ctx.tokenFile == null) ''
        printf 'token = "%s"\n' '${ctx.token}' >> "$config_path"
      ''}
      printf '\n' >> "$config_path"
    '')
    cfg.settings.contexts);
  in ''
    config_path="${config.xdg.configHome}/funnel/config.toml"
    $DRY_RUN_CMD mkdir -p "$(dirname "$config_path")"
    printf 'current_context = "%s"\n\n' '${cfg.settings.currentContext}' > "$config_path"
    ${contextEntries}
    $DRY_RUN_CMD chmod 600 "$config_path"
  '';
in {
  options.programs.funnel = {
    enable = lib.mkEnableOption "funnel tunnel client";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.funnel-client;
      description = "The funnel client package to use.";
    };

    settings = {
      currentContext = lib.mkOption {
        type = lib.types.str;
        default = "default";
        description = "Active context name.";
      };

      contexts = lib.mkOption {
        type = lib.types.attrsOf (lib.types.submodule {
          options = {
            server = lib.mkOption {
              type = lib.types.str;
              description = "Server URL for this context.";
            };

            token = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              description = ''
                Auth token. Stored in the Nix store — use tokenFile instead
                for secrets managed by sops-nix or agenix.
              '';
            };

            tokenFile = lib.mkOption {
              type = lib.types.nullOr lib.types.path;
              default = null;
              description = ''
                Path to a file containing the auth token.
                Read at activation time, never enters the Nix store.
                Use with sops-nix: config.sops.secrets."funnel/token".path
              '';
            };
          };
        });
        default = {};
        description = ''
          Server contexts. When any context sets tokenFile, the config is
          generated at activation time (mutable file with 0600 permissions).
          Otherwise it's a read-only symlink to the Nix store.
        '';
      };
    };

    environment = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = {};
      example = {
        RUST_LOG = "debug";
      };
      description = ''
        Environment variables set in the user's session for the funnel client.
        Useful for FUNNEL_SERVER, FUNNEL_CONTEXT, RUST_LOG.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions =
      lib.mapAttrsToList (name: ctx: {
        assertion = !(ctx.token != null && ctx.tokenFile != null);
        message = "programs.funnel.settings.contexts.${name}: set token or tokenFile, not both.";
      })
      cfg.settings.contexts;

    home.packages = [cfg.package];

    home.sessionVariables = cfg.environment;

    # static config when no tokenFile is used (symlink to nix store)
    xdg.configFile."funnel/config.toml" = lib.mkIf (hasContexts && !hasTokenFiles) {
      source = tomlFormat.generate "funnel-config.toml" staticConfigContents;
    };

    # dynamic config when tokenFile is used (generated at activation, 0600)
    home.activation.generateFunnelConfig = lib.mkIf (hasContexts && hasTokenFiles)
      (lib.hm.dag.entryAfter ["writeBoundary" "sopsNix"] activationScript);
  };
}
