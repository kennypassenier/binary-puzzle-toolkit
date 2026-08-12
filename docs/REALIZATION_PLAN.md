# Realization Plan — binsolve

Approved 2026-08-12 (Phase 5 gate: all milestones agreed, L0 amended —
CI covers Ubuntu + Windows + Arch container for Kenny's Garuda; all
standing rules confirmed; hook config: everything blocks). Status table
updated after every milestone gate.

## Status

| Milestone | Features | Status |
|---|---|---|
| L0 · Walking skeleton | [meta] | done (2026-08-12, CI run 31565429775 green) |
| L1 · Grid, regions & format | K7, M3-core, K6-part, M4-seed | done (2026-08-12, commit 5239591, CI green) |
| L2 · Fixpoint + tier 1–2 + events | K3-part, K16-core | done (2026-08-12, commit 3d54b2c, CI green) |
| L3 · Tier 3–4 + strategy-only | K3, M1 | not started |
| L4 · Search, uniqueness, outcomes | K4, K5, K6 | not started |
| L5 · The five special types | K2a–K2e | not started |
| L6 · The CLI | K8–K12, K16, M3, M1-flag | not started |
| L7 · Performance & difficulty | K13, M2, (M5 if benches demand) | not started |
| L8 · Fuzzing | M7 | not started |
| L9 · The TUI | K15 | not started |

Desired-not-scheduled: K11 is inside L6; M5 conditional in L7; M6
(JSON) stays Later.

## Milestones

### L0 · Walking skeleton [meta]
Workspace: `binsolve-core` (lib, zero deps, forbid(unsafe)),
`binsolve` (CLI), `binsolve-tui` (TUI) — compiling, empty modules.
rust-toolchain.toml pin; MIT LICENSE; .gitattributes (fixtures
`-text`); CI: fmt + clippy(-D warnings) + full tests on ubuntu-latest,
windows-latest, and an archlinux:latest container job (Garuda
amendment); CI check that core `[dependencies]` stays empty; commit
hooks live (everything blocks + ID requirement).
**Exit:** CI green on all three jobs; ID-less or gate-failing commit
physically rejected.

### L1 · Grid, regions & the frozen format [K7, M3-core, K6-part, M4-seed]
AR2 Cell/Grid, AR3 Region+RuleSet, AR7 parser/serializer + regression
vectors, AR12 corpus format, --check validation logic in core, corpus
seeded (the six verified site puzzles + starter set per size).
**Exit:** round-trip property test; every malformed-input class errors
with remedy; corpus meta-test green.

### L2 · Fixpoint engine + tier 1–2 strategies + events [K3-part, K16-core]
AR4 strategy trait, fixpoint loop, AR8 observer (Deduced), tier 1–2
strategies, trace formatter (K16 steps).
**Exit:** C# README tier-1/2 scenarios as regression tests; easy corpus
puzzles solve strategy-only; trace snapshot green.

### L3 · Tier 3–4 strategies + strategy-only mode [K3, M1]
KeepLineUnique, VirtualLimitReached, FillPossibilities generalized to
regions; M1 API (solved | stuck-at-N%).
**Exit:** all README scenarios pass; strategy-only corpus sweep
documented per difficulty.

### L4 · Search, uniqueness & outcomes [K4, K5, K6]
AR5 DFS (cheap tiers in-search), AR6 outcomes incl. MultipleSolutions,
contradiction reasons, Guessed/Backtracked/SolutionFound events.
**Exit:** cascade scenario solves; two-solution grid reports both;
every standard corpus puzzle solves and proves unique.

### L5 · The five special types [K2a–K2e]
Tag → regions mapping; specials corpus ≥2 per type; region-boundary
unit tests; M3 --check sweep validates AR3's all-rules assumption.
**Exit:** all five types solve + verify against published solutions.

### L6 · The CLI [K8–K12, K16, M3, M1-flag]
clap interface: single-arg, batch 1:1 + markers, atomic --out (AR11),
exit codes, TTY pretty + stats, --explain (stderr/file), --check,
--no-backtrack.
**Exit:** E2E suite on the real binary — mixed batch, every exit code,
markers, stdout/file equivalence, CRLF input — green on all CI jobs.

### L7 · Performance & difficulty [K13, M2, (M5)]
Criterion benches; release-mode G5 threshold test (<1 s worst, <50 ms
typical, 1,000-batch <30 s); M2 grading calibrated vs site labels. M5
activated only if the batch bench misses target.
**Exit:** threshold test green in release on Kenny's PC; grading
correlation documented.

### L8 · Fuzzing [M7]
cargo-fuzz targets (parser: never panic; solver: always terminate),
≥1 h each on Garuda; findings become regression tests before fixes.
**Exit:** both targets 1 h crash-free.

### L9 · The TUI [K15]
binsolve-tui over the AR9 event log: single/multi puzzle replay,
adjustable speed, statistics.
**Exit:** render-model unit tests green; manual acceptance by Kenny.

## Order rationale
Format before everything (it is the contract); strategies before
search (search propagates through them); composites after both are
proven separately; CLI once the core API is stable; performance before
fuzzing (fuzz the tuned code); TUI last on a hardened, frozen event
stream.
