#!/usr/bin/env bash
# binsolve quality gates (Phase 5, H1: everything blocks).
# Called by check-commit.sh before every git commit; non-zero blocks.
set -euo pipefail

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
