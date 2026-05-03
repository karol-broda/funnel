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
