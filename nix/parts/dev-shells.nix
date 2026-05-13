{...}: {
  perSystem = {pkgs, ...}: let
    devModules = import ../. {inherit pkgs;};
  in {
    devShells = {
      inherit (devModules.shells) default nightly wasm ci;
    };
  };
}
