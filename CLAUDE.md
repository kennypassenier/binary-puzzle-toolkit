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
| Current phase | 3 · Tech choice |
| Last completed gate | Phase 2 features FROZEN — 13 essential, 7 desired, 2 later (2026-08-12) |
| Next gate | Phase 3 tech-choice decision form |
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
