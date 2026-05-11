{inputs, ...}: {
  perSystem = {
    pkgs,
    lib,
    system,
    craneLib,
    built,
    ...
  }: {
    checks =
      {
        clippy = craneLib.cargoClippy (built.commonArgs
          // {
            inherit (built) cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets";
          });
        fmt = craneLib.cargoFmt {inherit (built.commonArgs) src pname version;};
      }
      // lib.optionalAttrs pkgs.stdenv.isLinux {
        nixos-server = import ../tests/server.nix {
          inherit pkgs system;
          self = inputs.self;
        };
      };
  };
}
