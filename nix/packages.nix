{
  pkgs,
  craneLib,
}: let
  src = let
    sqlFilter = path: _type:
      builtins.match ".*/migrations/.*\\.sql$" path != null;
  in
    pkgs.lib.cleanSourceWith {
      src = ../.;
      filter = path: type:
        (craneLib.filterCargoSources path type) || (sqlFilter path type);
    };

  commonArgs = {
    inherit src;
    pname = "funnel";
    version = "0.1.0";
    strictDeps = true;

    buildInputs =
      [pkgs.openssl pkgs.zlib]
      ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
        pkgs.darwin.apple_sdk.frameworks.Security
        pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
        pkgs.darwin.apple_sdk.frameworks.CoreFoundation
        pkgs.darwin.apple_sdk.frameworks.CoreServices
        pkgs.libiconv
      ];

    nativeBuildInputs = [pkgs.pkg-config];

    OPENSSL_NO_VENDOR = "1";
    OPENSSL_DIR = "${pkgs.openssl.dev}";
    OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
    OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
  };

  cargoArtifacts = craneLib.buildDepsOnly (commonArgs
    // {
      pname = "funnel-workspace";
    });
in {
  inherit src commonArgs cargoArtifacts;

  funnel-server = craneLib.buildPackage (commonArgs
    // {
      inherit cargoArtifacts;
      pname = "funnel-server";
      cargoExtraArgs = "--package funnel-server";
      doCheck = false;

      meta = {
        description = "Funnel tunnel server";
        mainProgram = "funnel-server";
      };
    });

  funnel-client = craneLib.buildPackage (commonArgs
    // {
      inherit cargoArtifacts;
      pname = "funnel-client";
      cargoExtraArgs = "--package funnel-client";
      doCheck = false;

      postInstall = ''
        ln -s $out/bin/funnel-client $out/bin/funnel
      '';

      meta = {
        description = "Funnel tunnel client";
        mainProgram = "funnel";
      };
    });
}
