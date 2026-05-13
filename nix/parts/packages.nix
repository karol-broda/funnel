{...}: {
  perSystem = {built, ...}: {
    packages = {
      inherit (built) funnel-server funnel-client;
      default = built.funnel-client;
    };
  };
}
