{pkgs}: let
  toolchains = import ./toolchains.nix {inherit pkgs;};
  buildInputs = import ./build-inputs.nix {inherit pkgs;};
  devTools = import ./dev-tools.nix {inherit pkgs;};
  shells = import ./shells.nix {inherit pkgs toolchains buildInputs devTools;};
in {inherit toolchains buildInputs devTools shells;}
