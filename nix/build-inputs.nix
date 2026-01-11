{pkgs}: {
  buildInputs = with pkgs;
    [
      openssl
      zlib
    ]
    ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
      darwin.apple_sdk.frameworks.Security
      darwin.apple_sdk.frameworks.SystemConfiguration
      darwin.apple_sdk.frameworks.CoreFoundation
      darwin.apple_sdk.frameworks.CoreServices
      libiconv
    ];

  nativeBuildInputs = with pkgs; [
    pkg-config
  ];

  env = {
    OPENSSL_NO_VENDOR = "1";
    OPENSSL_DIR = "${pkgs.openssl.dev}";
    OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
    OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
  };
}
