{...}: {
  perSystem = {
    pkgs,
    built,
    ...
  }: let
    containers = import ../containers.nix {
      inherit pkgs;
      packages = built;
    };
  in {
    packages = {
      inherit (containers) funnel-server-image funnel-client-image;
    };
  };
}
