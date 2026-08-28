#!/usr/bin/env bash
# binforge quality gates (Phase 5, H1).
# Called by .githooks/pre-commit and by the Claude Code PreToolUse hook
# before every git commit; non-zero blocks.
# Benchmarks (K11: the G8 baseline harness) and the full D1 validation
# (100 puzzles per size through the real binsolve binary) run in CI, not
# here: they take minutes and would make every commit painful.
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

# Before L0 the workspace has no manifest yet, so the cargo gates have
# nothing to run against. Announced, never silent (standing rule 12):
# this branch disappears the moment L0 creates Cargo.toml.
if [ ! -f Cargo.toml ]; then
  echo "gates: no Cargo.toml yet (pre-L0) — cargo gates skipped, purity check still runs"
else
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets -- -D warnings
  cargo test --workspace
fi

./.claude/hooks/core-purity.sh
