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
| Current phase | 10 done — all eleven phases complete. v1.0.0 released 2026-08-30 from commit 62f971b |
| Last completed gate | phase 10 retrospective (2026-08-30): six lessons adopted, committed to dev-procedure as 000c03e |
| Next gate | none — the project is released. New work starts a mini-round or a new cycle |
| AFK mode | OFF since 2026-08-29 — the queue was presented and cleared |

<!-- Update this block after every completed gate. -->

## Layout

| Crate | Holds |
|---|---|
| `bpt-core` | grid, regions, rules, the text format, strategies, search — zero runtime dependencies |
| `bpt-forge` | geometry, fill, carve, grading: the generator |
| `bpt-tui` | the replay viewer's render model |
| `bpt` | one binary: `bpt solve`, `bpt forge`, `bpt watch`, `bpt inspect` |

## Project documents

| Doc | Purpose |
|---|---|
| docs/ID_MAP.md | how the generator's feature IDs were renumbered at the merge |
| docs/MERGE_PLAN.md | the merge decisions and the order of work |
| docs/USER_GUIDE.md, DEBUGGING_GUIDE.md, OPERATIONS_RUNBOOK.md, ARCHITECTURE_REFERENCE.md, TEST_PLAN.md | the current user documentation, covering both halves |
| docs/solve/ | the solver half's phase documents (scope, features, decisions, plan) |
| docs/legacy/ | the solver's pre-merge user docs, superseded but kept |
| docs/forge/ | the generator half's phase documents |
| docs/DEVELOPMENT.md | working on the code; the one-time hook activation |
| docs/TEST_PLAN.md | every suite, and what is deliberately not covered |

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
