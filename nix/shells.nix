{
  pkgs,
  toolchains,
  buildInputs,
  devTools,
}: let
  mkRustShell = {
    toolchain,
    includeDevTools ? true,
    extraPackages ? [],
    extraBuildInputs ? [],
    extraEnv ? {},
    shellHookExtra ? "",
  }:
    pkgs.mkShell ({
        packages =
          [toolchain]
          ++ buildInputs.nativeBuildInputs
          ++ pkgs.lib.optionals includeDevTools devTools
          ++ extraPackages;

        buildInputs = buildInputs.buildInputs ++ extraBuildInputs;

        RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";

        shellHook = ''
          if [ -n "$PS1" ]; then
            echo "rust dev shell"
            echo "  rustc: $(rustc --version)"
            echo "  cargo: $(cargo --version)"
          fi
          ${shellHookExtra}
        '';
      }
      // buildInputs.env // extraEnv);
in {
  default = mkRustShell {
    toolchain = toolchains.default;
    extraPackages = with pkgs; [
      tailwindcss
      binaryen
      nodejs
    ];
  };

  ci = mkRustShell {
    toolchain = toolchains.default;
    includeDevTools = false;
    extraPackages = with pkgs; [
      cargo-nextest
    ];
  };

  inherit mkRustShell;
}
