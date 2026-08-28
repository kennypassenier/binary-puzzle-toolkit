# binforge

Generator for binary puzzles (Takuzu/Binairo) in Rust — any even-sized
standard grid, the five special/composite types from binarypuzzle.com,
and invented composite types, each with a proven-unique solution and a
technique-graded difficulty. Companion to `~/Projects/binsolve`
(solver); successor to the archived C# `BinaryPuzzleScraper`.

This project follows the dev procedure in `~/Projects/dev-procedure/`
(`/project-flow`). Standing rules apply to every change:
`~/Projects/dev-procedure/STANDING_RULES.md`.
**Open sessions in THIS directory** — hooks and repo-scoped tools only
work here (standing rule 19).

## Procedure status

| Field | Value |
|---|---|
| Current phase | 6 · Development loop (L1 in progress) |
| Last completed gate | L0 milestone report signed off (2026-08-12) |
| Next gate | Combined AFK milestone report (L1 onwards) |
| AFK mode | **on** (2026-08-12) — milestone gates accumulate; deviations quarantine + queue as mini-rounds |

<!-- Update this block after every completed gate. -->

## Project documents

| Doc | Purpose |
|---|---|
| docs/SCOPE.md | goals, non-goals, success criteria, constraints (Phase 0) |
| docs/FEATURES.md | rated feature list with permanent IDs (Phase 2) |
| docs/ARCHITECTURE_DECISIONS.md | frozen AR decisions incl. tech choice (Phases 3-4) |
| docs/REALIZATION_PLAN.md | milestones + status table (Phase 5) |
| docs/TEST_PLAN.md | what is proven where + accepted limitations (Phase 7) |

## Gates (enforced)

Two layers, both running `.claude/hooks/gates.sh` (fmt, clippy with
warnings as errors, full test suite, core-purity check):

- **git-native** — `.githooks/pre-commit` + `.githooks/commit-msg` via
  `core.hooksPath`, so the gates hold from any terminal or tool.
- **session** — `.claude/hooks/check-commit.sh` as a second layer.

A commit is refused unless the gates pass and the message carries IDs
in brackets (`[K2]`, `[L4]`, `[meta]`). CI re-runs the same gates on
Linux and Windows; branch protection on `main` requires them.
