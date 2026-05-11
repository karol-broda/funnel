{
  description = "funnel | expose local services through secure tunnels";

  nixConfig = {
    extra-substituters = ["https://cache.karolbroda.com/funnel"];
    extra-trusted-public-keys = ["funnel:f2v8GXhe4t/c5ITHJ/LRYqcen5RWsYNEa9BKvxxxVBA="];
  };

  inputs = {
    nixpkgs = {
      url = "github:NixOS/nixpkgs/nixos-25.05";
    };
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
    };
    systems = {
      url = "github:nix-systems/default";
    };
  };

  outputs = inputs @ {flake-parts, ...}:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = import inputs.systems;
      imports = [./nix/parts];
    };
}
