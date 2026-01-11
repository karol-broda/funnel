{pkgs}: {
  default = pkgs.rust-bin.stable.latest.default.override {
    extensions = [
      "rust-src"
      "rust-analyzer"
      "clippy"
      "rustfmt"
    ];
  };

  nightly = pkgs.rust-bin.nightly.latest.default.override {
    extensions = [
      "rust-src"
      "rust-analyzer"
      "clippy"
      "rustfmt"
      "miri"
      "llvm-tools"
    ];
  };

  wasm = pkgs.rust-bin.stable.latest.default.override {
    extensions = [
      "rust-src"
      "rust-analyzer"
      "clippy"
      "rustfmt"
    ];
    targets = [
      "wasm32-unknown-unknown"
      "wasm32-wasip1"
    ];
  };

  custom = {
    channel ? "stable",
    version ? "latest",
    extensions ? [],
    targets ? [],
  }: let
    base =
      if version == "latest"
      then pkgs.rust-bin.${channel}.latest.default
      else pkgs.rust-bin.${channel}.${version}.default;
  in
    base.override {
      inherit extensions targets;
    };
}
