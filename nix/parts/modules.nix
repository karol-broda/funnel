{inputs, ...}: {
  flake = {
    nixosModules = {
      funnel-server = import ../nixos/module.nix inputs.self;
      default = inputs.self.nixosModules.funnel-server;
    };

    homeManagerModules = {
      funnel-client = import ../home-manager/module.nix inputs.self;
      default = inputs.self.homeManagerModules.funnel-client;
    };

    overlays.default = final: prev: {
      funnel-server = inputs.self.packages.${prev.system}.funnel-server;
      funnel-client = inputs.self.packages.${prev.system}.funnel-client;
    };
  };
}
