#!/usr/bin/env bash
set -euo pipefail
cargo build -p funnel-server -p funnel-client
cargo test -p funnel-e2e -- --nocapture "$@"
