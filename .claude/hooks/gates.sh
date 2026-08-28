#!/usr/bin/env bash
# binforge quality gates (Phase 5, H1).
# Called by check-commit.sh before every git commit; non-zero blocks.
# Benchmarks (K11) and the full D1 validation run in CI, not here:
# they take minutes and would make every commit painful.
set -euo pipefail

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./.claude/hooks/core-purity.sh
