{pkgs}:
with pkgs; [
  cargo-watch
  cargo-edit
  cargo-expand
  cargo-outdated
  cargo-audit
  cargo-nextest
  cargo-deny
  cargo-machete
  cargo-flamegraph
  cargo-bloat

  bacon
  just
  hyperfine
  tokei
]
