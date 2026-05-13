{inputs, ...}: {
  perSystem = {
    pkgs,
    lib,
    system,
    ...
  }: {
    checks = lib.optionalAttrs pkgs.stdenv.isLinux {
      nixos-server = import ../tests/server.nix {
        inherit pkgs system;
        self = inputs.self;
      };
    };
  };
}
