{
  pkgs,
  self,
  system,
}:
pkgs.nixosTest {
  name = "funnel-server";

  nodes = {
    server = {
      config,
      lib,
      ...
    }: {
      imports = [self.nixosModules.funnel-server];

      services.funnel.server = {
        enable = true;
        port = 8080;
        quicPort = 4433;
        seedApiKey = true;
      };

      environment.systemPackages = [pkgs.jq];
      networking.firewall.allowedTCPPorts = [8080];
      networking.firewall.allowedUDPPorts = [4433];
    };

    client = {lib, ...}: {
      environment.systemPackages = [
        self.packages.${system}.funnel-client
        pkgs.curl
      ];
    };
  };

  testScript = ''
    import json

    server.start()
    server.wait_for_unit("funnel-server.service")
    server.wait_for_open_port(8080)

    # health endpoint
    health = json.loads(server.succeed("curl -sf http://localhost:8080/api/v1/health"))
    assert health["status"] == "healthy", f"expected healthy, got {health['status']}"
    assert health["uptime_secs"] >= 0, "uptime should be non-negative"

    # info endpoint returns correct protocol version and quic port
    info = json.loads(server.succeed("curl -sf http://localhost:8080/api/v1/info"))
    assert info["version"] == 1, f"expected version 1, got {info['version']}"
    assert info["quic_port"] == 4433, f"expected quic_port 4433, got {info['quic_port']}"

    # unauthenticated requests to protected endpoints return 401
    status = server.succeed("curl -s -o /dev/null -w '%{http_code}' http://localhost:8080/api/v1/tunnels").strip()
    assert status == "401", f"expected 401, got {status}"

    # seed api key is emitted as a structured log field
    seed_key = server.succeed(
        "journalctl -u funnel-server --output=cat"
        " | grep '^{'"
        " | jq -r 'select(.fields.seed_api_key != null) | .fields.seed_api_key'"
        " | head -1"
    ).strip()
    assert len(seed_key) >= 40, f"seed key too short ({len(seed_key)} chars): {seed_key}"

    # authenticated request to /me succeeds with the seed key
    me_status = server.succeed(
        f"curl -s -o /dev/null -w '%{{http_code}}' -H 'Authorization: Bearer {seed_key}' http://localhost:8080/api/v1/me"
    ).strip()
    assert me_status == "200", f"expected 200 from /me, got {me_status}"

    # service runs as dynamic user (not root)
    server.succeed("systemctl show funnel-server -p DynamicUser | grep -q 'yes'")

    # state directory exists
    server.succeed("test -d /var/lib/funnel")

    # service restarts after kill
    server.succeed("systemctl kill -s KILL funnel-server")
    server.wait_for_unit("funnel-server.service")
    server.wait_for_open_port(8080)

    # client can reach server over the network
    client.start()
    client.wait_for_unit("multi-user.target")

    client_health = json.loads(client.succeed("curl -sf http://server:8080/api/v1/health"))
    assert client_health["status"] == "healthy"

    # client context creation works
    client.succeed("funnel context create test --server http://server:8080")
    client.succeed("test -f /root/.config/funnel/config.toml")
  '';
}
