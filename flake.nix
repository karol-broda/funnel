{
  description = "go 1.25.0 dev flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.05";
    systems.url = "github:nix-systems/default";
  };

  outputs = { self, nixpkgs, systems }:
    let
      supportedSystems = import systems;
      forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f system);
    in
    {
      overlays.default = final: prev:
        let
          version = "1.25.0";

          platformInfo = {
            "x86_64-linux" = { suffix = "linux-amd64"; sri = "sha256-KFKvDLIKExObNEiZLmm4aOUO0Pih5ZQO4d6eGaEjthM="; };
            "aarch64-linux" = { suffix = "linux-arm64"; sri = "sha256-Bd511plKJ4NpmBXuVTvVqTJ9i3mZHeNuOLZoYngvVK4="; };
            "i686-linux" = { suffix = "linux-386"; sri = "sha256-jGAt2dmbyUU7OZXSDOS684LMUIVZAKDs5d6ZKd9KmTo="; };
            "armv6l-linux" = { suffix = "linux-armv6l"; sri = "sha256-paj4GY/PAOHkhbjs757gIHeL8ypAik6Iczcb/ORYzQk="; };

            "x86_64-darwin" = { suffix = "darwin-amd64"; sri = "sha256-W9YOgjA3BiwjB8cegRGAmGURZxTW9rQQWXz1B139gO8="; };
            "aarch64-darwin" = { suffix = "darwin-arm64"; sri = "sha256-VEkyhEFW2Bcveij3fyrJwVojBGaYtiQ/YzsKCwDAdJw="; };
          };

          hostSystem = prev.stdenv.hostPlatform.system;

          chosen =
            if prev.lib.hasAttr hostSystem platformInfo then platformInfo.${hostSystem}
            else
              throw ''
                unsupported system: ${hostSystem}
                add a mapping for your platform using the upstream tarball + sri sha256
              '';
        in
        {
          go_1_25_bin = prev.stdenvNoCC.mkDerivation {
            pname = "go";
            version = version;

            src = prev.fetchurl {
              url = "https://go.dev/dl/go${version}.${chosen.suffix}.tar.gz";
              hash = chosen.sri;
            };

            dontBuild = true;

            installPhase = ''
              runHook preInstall
              mkdir -p "$out"/{bin,share}
              tar -C "$TMPDIR" -xzf "$src"
              cp -a "$TMPDIR/go" "$out/share/go"
              ln -s "$out/share/go/bin/go"    "$out/bin/go"
              ln -s "$out/share/go/bin/gofmt" "$out/bin/gofmt"
              runHook postInstall
            '';

            dontPatchELF = true;
            dontStrip = true;

            meta = with prev.lib; {
              description = "go compiler and tools v${version}";
              homepage = "https://go.dev/dl/";
              license = licenses.bsd3;
              platforms = [ hostSystem ];
            };
          };
        };

      packages = forAllSystems (system:
        let pkgs = import nixpkgs { inherit system; overlays = [ self.overlays.default ]; };
        in {
          default = pkgs.go_1_25_bin;
          go_1_25_bin = pkgs.go_1_25_bin;
        }
      );

      devShells = forAllSystems (system:
        let pkgs = import nixpkgs { inherit system; overlays = [ self.overlays.default ]; };
        in {
          default = pkgs.mkShell {
            packages = [ pkgs.go_1_25_bin pkgs.git ];

            GOTOOLCHAIN = "local";

            shellHook = ''
              echo "go toolchain: $(go version)"
            '';
          };
        }
      );
    };
}
