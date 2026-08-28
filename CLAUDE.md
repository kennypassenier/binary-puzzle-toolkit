# BinaryPuzzleToolkit

One toolkit for binary puzzles (Takuzu/Binairo): it **solves** them —
every standard size plus the five composite types from
binarypuzzle.com, with a uniqueness proof — and **generates** them.
Merged from the separate binsolve and binforge projects on 2026-08-28.

This project follows the dev procedure in `~/Projects/dev-procedure/`
(`/project-flow`). Standing rules apply to every change:
`~/Projects/dev-procedure/STANDING_RULES.md`.

## Procedure status

| Field | Value |
|---|---|
| Current phase | solver half: 9 (release prepared, not tagged); generator half: 6 (L1-L4 done, L5 partly) |
| Last completed gate | merge decisions + phase 2 mandatory items (2026-08-28) |
| Next gate | combined report: the merge and the generator's L2-L5 |
| AFK mode | ON since 2026-08-28 — gates accumulate, deviations queue as mini-rounds |

<!-- Update this block after every completed gate. -->

## Layout

| Crate | Holds |
|---|---|
| `bpt-core` | grid, regions, rules, the text format, strategies, search — zero runtime dependencies |
| `bpt-forge` | geometry, fill, carve, grading: the generator |
| `bpt-tui` | the replay viewer's render model |
| `bpt` | one binary: `bpt solve`, `bpt forge`, `bpt watch` |

## Project documents

| Doc | Purpose |
|---|---|
| docs/ID_MAP.md | how the generator's feature IDs were renumbered at the merge |
| docs/MERGE_PLAN.md | the merge decisions and the order of work |
| docs/solve/ | the solver half's phase documents and its user documentation |
| docs/forge/ | the generator half's phase documents |
| docs/DEVELOPMENT.md | working on the code; the one-time hook activation |

## Feature IDs

The two halves numbered independently, so every ID once collided. The
solver keeps K1–K16, M1–M7, AR1–AR13, T1–T12; the generator's shifted to
K20–K31, M20–M30, AR20–AR32. See docs/ID_MAP.md — a generator commit
from before the merge saying `[K3]` means what is now K22.

## Enforcement layers

Two layers, both live:

1. **Git-native (primary, holds from any session/terminal):**
   `.githooks/pre-commit` runs `.claude/hooks/gates.sh` (fmt, clippy
   with warnings-as-errors, full test suite); `.githooks/commit-msg`
   requires feature IDs in brackets. Wired via
   `git config core.hooksPath .githooks` — a fresh clone must run that
   once.
2. **Claude Code PreToolUse hook** (`.claude/settings.json` →
   `check-commit.sh`), which only loads in sessions opened in this
   directory.
