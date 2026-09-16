#!/usr/bin/env bash
# binsolve quality gates (Phase 5, H1: everything blocks).
# Called by check-commit.sh before every git commit; non-zero blocks.
set -euo pipefail
# Kenny, 2026-09-16, standing rule 49 (commit-floor and rust-suite):
# format and lint always run, and the suite is skipped when no Rust
# source moved. Measured across sixteen projects: 41% of commits touch
# only documentation or configuration and paid for the suite anyway. Per
# crate was measured and rejected — `cargo test -p <crate>` is not faster
# than the whole workspace, because cargo runs every test binary either
# way.
. "$(git rev-parse --show-toplevel)/.githooks/gate-cache.sh"


cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
gate_glob suite '*.rs' 'Cargo.toml' 'Cargo.lock' '*/Cargo.toml' -- \
  cargo test --workspace

gate_cache_done
