#!/usr/bin/env bash
# Everything the CI workflow ran until 2026-09-29, when Kenny moved every
# build and check to his own machine ("alle builds lokaal"):
#
#   scripts/check.sh            # WINDOWS_TESTS=skip to go without Windows
#
# The CI matrix had three legs. Arch: this machine (WSL Arch or Garuda) is
# that leg. Ubuntu: the release builds the Linux binary in rust:1-bookworm,
# so a glibc surprise shows there. Windows: the whole suite, cross-built with
# cargo-xwin and run on real Windows through WSL interop
# (scripts/windows-tests.sh); on Garuda it refuses unless WINDOWS_TESTS=skip.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
echo "[1/2] gates: fmt, clippy -D warnings, tests"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
echo "[2/2] the suite on Windows"
if [ "${WINDOWS_TESTS:-}" = skip ]; then
  echo "  SKIPPED on request (WINDOWS_TESTS=skip)"
else
  scripts/windows-tests.sh --workspace || {
    rc=$?; [ $rc -eq 3 ] && echo "  no Windows here; rerun on WSL, or WINDOWS_TESTS=skip to go without" >&2; exit $rc; }
fi
echo "check passed"
