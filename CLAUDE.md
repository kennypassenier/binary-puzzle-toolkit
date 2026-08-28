# binsolve

Complete solver for binary puzzles (Takuzu/Binairo) in Rust — all
standard n×n sizes plus the five special/composite types from
binarypuzzle.com, with uniqueness proving. Rewrite of the archived C#
`BinaryPuzzleSolver`.

This project follows the dev procedure in `~/Projects/dev-procedure/`
(`/project-flow`). Standing rules apply to every change:
`~/Projects/dev-procedure/STANDING_RULES.md`.
**Open sessions in THIS directory** — hooks and repo-scoped tools only
work here (standing rule 19).

## Procedure status

| Field | Value |
|---|---|
| Current phase | 6 · Development loop (L8 runs in progress, L9 built; gates pending) |
| Last completed gate | L7 milestone report (2026-08-12) |
| Next gate | L8 milestone report (after fuzz runs), then L9 |
| AFK mode | off |

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

Commits are blocked by `.claude/hooks/check-commit.sh` unless
`.claude/hooks/gates.sh` passes and the message carries IDs in
brackets (`[W12]`, `[L4b]`, `[meta]`). CI re-runs the same gates on
every push; red blocks merge.

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
