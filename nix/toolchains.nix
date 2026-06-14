{pkgs}: {
  default = pkgs.rust-bin.stable.latest.default.override {
    extensions = [
      "rust-src"
      "rust-analyzer"
      "clippy"
      "rustfmt"
    ];
    targets = [
      "wasm32-unknown-unknown"
    ];
  };
}
