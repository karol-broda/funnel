{
  pkgs,
  packages,
}: let
  user = "funnel";
  uid = "1000";
  gid = "1000";

  passwdFile = pkgs.writeTextDir "etc/passwd" ''
    root:x:0:0:root:/root:/bin/false
    ${user}:x:${uid}:${gid}:${user}:/home/${user}:/bin/false
  '';

  groupFile = pkgs.writeTextDir "etc/group" ''
    root:x:0:
    ${user}:x:${gid}:
  '';

  nssSwitchFile = pkgs.writeTextDir "etc/nsswitch.conf" ''
    hosts: files dns
  '';

  dataDir = pkgs.runCommand "funnel-data-dirs" {} ''
    mkdir -p $out/var/lib/funnel/certs
    mkdir -p $out/tmp
  '';

  homeDir = pkgs.runCommand "funnel-home-dirs" {} ''
    mkdir -p $out/home/${user}/.config/funnel
  '';
in {
  funnel-server-image = pkgs.dockerTools.buildLayeredImage {
    name = "funnel-server";
    tag = "latest";

    contents = [
      packages.funnel-server
      pkgs.cacert
      pkgs.tzdata
      passwdFile
      groupFile
      nssSwitchFile
      dataDir
    ];

    config = {
      Cmd = ["${packages.funnel-server}/bin/funnel-server"];
      User = "${uid}:${gid}";
      WorkingDir = "/var/lib/funnel";
      ExposedPorts = {
        "8080/tcp" = {};
        "8443/tcp" = {};
        "4433/udp" = {};
      };
      Env = [
        "FUNNEL_HOST=0.0.0.0"
        "FUNNEL_PORT=8080"
        "FUNNEL_TLS_PORT=8443"
        "FUNNEL_QUIC_PORT=4433"
        "FUNNEL_CERT_DIR=/var/lib/funnel/certs"
        "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
        "TZDIR=${pkgs.tzdata}/share/zoneinfo"
      ];
      Volumes = {
        "/var/lib/funnel" = {};
      };
      Labels = {
        "org.opencontainers.image.source" = "https://github.com/karol-broda/funnel";
        "org.opencontainers.image.description" = "Funnel tunnel server";
      };
    };
  };

  funnel-client-image = pkgs.dockerTools.buildLayeredImage {
    name = "funnel-client";
    tag = "latest";

    contents = [
      packages.funnel-client
      pkgs.cacert
      passwdFile
      groupFile
      nssSwitchFile
      homeDir
    ];

    config = {
      Entrypoint = ["${packages.funnel-client}/bin/funnel"];
      User = "${uid}:${gid}";
      Env = [
        "SSL_CERT_FILE=${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
      ];
      Labels = {
        "org.opencontainers.image.source" = "https://github.com/karol-broda/funnel";
        "org.opencontainers.image.description" = "Funnel tunnel client";
      };
    };
  };
}
