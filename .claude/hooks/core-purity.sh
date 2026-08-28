#!/usr/bin/env bash
# AR1: binforge-core has zero ambient I/O. std::time is banned whole —
# Instant, not just SystemTime, is what a time budget would reach for,
# and a wall-clock bound in core would falsify M1 and M3 (AR7).
set -euo pipefail

src="binforge-core/src"
[ -d "$src" ] || exit 0

violations=$(grep -rnE 'std::fs|std::thread|std::time|println!|eprintln!|print!|eprint!' "$src" || true)

if [ -n "$violations" ]; then
  echo "core-purity: binforge-core must have zero ambient I/O (AR1)." >&2
  echo "$violations" >&2
  echo "" >&2
  echo "Remedy: move the I/O to the binforge binary, or pass the value in as a parameter." >&2
  exit 1
fi
