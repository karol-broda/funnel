{
  description = "rust development flake";

  inputs = {
    nixpkgs = {
      url = "github:NixOS/nixpkgs/nixos-25.05";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    systems = {
      url = "github:nix-systems/default";
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    systems,
  }: let
    supportedSystems = import systems;
    forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f system);

    mkPkgs = system:
      import nixpkgs {
        inherit system;
        overlays = [
          rust-overlay.overlays.default
        ];
      };

    mkModules = system:
      import ./nix {
        pkgs = mkPkgs system;
      };
  in {
    devShells = forAllSystems (
      system: let
        modules = mkModules system;
      in {
        inherit (modules.shells) default nightly wasm;
      }
    );

    overlays = {
      default = rust-overlay.overlays.default;
    };
  };
}
