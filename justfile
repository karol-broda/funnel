default:
    @just --list

# build all crates
build:
    cargo build --workspace

# build in release mode
release:
    cargo build --workspace --release

# run all unit tests with nextest
test *args:
    cargo nextest run --workspace {{ args }}

# run e2e tests (builds server + client first)
e2e *args:
    cargo build -p funnel-server -p funnel-client
    cargo test -p funnel-e2e {{ args }}

# run clippy and check formatting
lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all -- --check

# format all code
fmt:
    cargo fmt --all

# watch for changes and rebuild (requires cargo-watch)
dev:
    bacon

# run clippy with auto fix
fix:
    cargo clippy --workspace --all-targets --fix --allow-dirty
    cargo fmt --all

# check that everything compiles without building
check:
    cargo check --workspace --all-targets

# audit dependencies for security vulnerabilities
audit:
    cargo audit

# find unused dependencies
machete:
    cargo machete

# generate openapi.json for the docs site
gen-openapi:
    cargo run -p funnel-server -- --dump-openapi > docs/openapi.json

# build nix packages
nix-build:
    nix build .#funnel-server .#funnel-client

# build oci container images
nix-images:
    nix build .#funnel-server-image .#funnel-client-image

# run nix checks (clippy, fmt, nixos vm test)
nix-check:
    nix flake check

# push all outputs to the attic binary cache
nix-push:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "building and pushing funnel-server..."
    nix build .#funnel-server --no-link --print-out-paths | xargs attic push funnel
    echo "building and pushing funnel-client..."
    nix build .#funnel-client --no-link --print-out-paths | xargs attic push funnel
    echo "done"

# push container images to attic binary cache
nix-push-images:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "building and pushing funnel-server-image..."
    nix build .#funnel-server-image --no-link --print-out-paths | xargs attic push funnel
    echo "building and pushing funnel-client-image..."
    nix build .#funnel-client-image --no-link --print-out-paths | xargs attic push funnel
    echo "done"

# load a container image into docker
nix-docker-load target="funnel-server-image":
    nix build .#{{ target }} && docker load < result
