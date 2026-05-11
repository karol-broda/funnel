{inputs, ...}: {
  perSystem = {system, ...}: let
    pkgs = import inputs.nixpkgs {
      inherit system;
      overlays = [inputs.rust-overlay.overlays.default];
    };
    toolchain = pkgs.rust-bin.stable.latest.default;
    craneLib = (inputs.crane.mkLib pkgs).overrideToolchain toolchain;
    built = import ../packages.nix {inherit pkgs craneLib;};
  in {
    _module.args = {inherit pkgs craneLib built;};
  };
}
