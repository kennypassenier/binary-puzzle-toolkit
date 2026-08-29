# Realization Plan — binforge

Phase 5 outcome, approved by Kenny 2026-08-12 (all milestones agreed
unchanged, standing rules confirmed, hook configuration as proposed,
AFK mode off). Enforcement was installed before L0 per the procedure.

## Status

| Milestone | Features | Status |
|---|---|---|
| L0 · Walking skeleton, CI green | [meta] | **done** (signed off 2026-08-12) |
| L1 · Geometry model + inspect | K23, M25, AR22b, AR32.4, AR32.5 | **built + reviewed, gate pending (AFK)** |
| L2 · solver changes B1–B3 | AR21, AR22b | **done** (2026-08-28, dissolved by the merge: rectangular regions, choice oracle, custom geometries) |
| L3 · Random solution filler | K20, M20, AR21, AR28, AR32.1 | **done** (2026-08-28; cell choice replaced 2026-08-29 — uniform sampling was pathological on the composites, see K27's commit) |
| L4 · Carve loop + grading + targeting | K21, K24, K25, AR23, AR31 | **done** (2026-08-28, tier-ceiling carve, verified through the solver) |
| L5 · Output, batches, CLI | K26, K27, K29, M21, M23, M26, AR27, AR29, AR30 | **done except cancellation** (2026-08-29): `--out-dir` writes corpus files, the flat validation file and a manifest; duplicates refused, shortfalls reported, failed batches discarded, progress on a terminal only. M26's Ctrl-C needs a signal-handling dependency — queued as Q5 |
| L6 · Independent validation harness + restore drill | K31, D1, M30, AR27, B5 | not started |
| L7 · Parallelism, benchmarks, baselines | M22, K30, AR25, B4 | not started |
| L8 · Invented types + property testing | K28, D4, M28, M24 | not started |

Remaining after L8: K22 (the five special types) is covered
incrementally — its geometries land in L1, its generation in L3/L4, its
validation batches in L6.

## Ordering rationale

- **L1 before L2** — the geometry model needs nothing from binsolve, so
  it proceeds while the blocking mini-rounds are pending instead of
  idling behind them, and M25's inspect command is the debugging tool
  for every milestone after it.
- **L2 is external.** B1 (choice oracle) blocks K20 and therefore
  everything downstream; B2/B3 (custom + rectangular geometry) block
  the invented types in L8. Those sessions run in `~/Projects/binsolve`,
  where its hooks and status block live.
- **L4 is the risky milestone** — the carve loop is where the measured
  performance surprises live, which is why AFK mode is off and L7's
  baselines come after it.
- **L6 before L7** — prove correctness independently before optimizing;
  a fast generator that emits ambiguous puzzles is worthless.

## Exit criteria

### L0
CI green on Linux and Windows; the core-purity check demonstrably fails
when a `println!` is added to core.

### L1
All built-in geometries load and render correctly (insta snapshots);
malformed geometry names the offending region; K23's
error-vs-infeasible distinction covered by tests.

### L2
The three amendments recorded in binsolve's docs with its own suite
green; binforge pins the resulting rev.

### L3
Filled grids valid for every built-in geometry (property test); same
seed ⇒ byte-identical, different seeds ⇒ different; every completion
independently validated (AR32.1); infeasible geometry rejected within
its work-unit bound.

### L4
Every emitted puzzle independently re-proven unique; measured level
equals requested level for L1–L3 across standard sizes; L4 reachability
documented per geometry; unreachable level returns the explicit error
rather than a mislabel.

### L5
A generated line solves in the pinned binsolve binary unmodified;
interrupted run leaves no torn files; cancellation yields
`status: cancelled` plus non-zero exit; duplicate re-rolls resolve in
index order; exit-code table covered.

### L6
D1's batches (100 per standard size 6–20, per special type) all confirm
exactly one solution through the real binsolve binary; the sabotage
puzzle is caught; a mismatched binary is detected, never silently used. M30's
restore drill passes: a fresh clone plus a stored seed reproduces a
previous batch byte for byte.

### L7
Parallel output identical to sequential for the same seed; baseline
file committed with measured medians and p95s — **this is where scope
G8's provisional figures become real numbers**.

### L8
Both invented types generate valid unique puzzles and are solved
end-to-end by the pinned binsolve binary; property tests shrink
failures to a reproducing `(seed, index, attempt)`.

## Enforcement (installed 2026-08-12, before L0)

- `.claude/hooks/gates.sh` — `cargo fmt --check`, `cargo clippy -D
  warnings`, `cargo test --workspace`, and `core-purity.sh`. Blocks the
  commit on failure.
- `.claude/hooks/check-commit.sh` — from `~/Projects/dev-procedure`;
  requires feature IDs in brackets in the commit message.
- `.claude/hooks/core-purity.sh` — AR20: no `std::fs`, `std::thread`,
  `std::time`, or print macros in `binforge-core`.
- `.githooks/pre-commit` + `.githooks/commit-msg`, wired via
  `core.hooksPath` — **git-native**, so the gates hold from any session,
  tool or terminal; the Claude Code PreToolUse hook is a second layer,
  not the only one (procedure amendment 2026-08-12). Verified by a
  deliberately ID-less commit being refused.
- `.github/workflows/ci.yml` — the same gates on Linux and Windows,
  plus the purity job. Red blocks merge.
- Branch protection on `main` requiring the CI checks — set 2026-08-12.
  It was refused while the repository was private ("Upgrade to GitHub
  Pro or make this repository public"); the enforcement gate chose
  public, matching binsolve, which restored it.
- Deliberately **not** blocking commits: criterion benchmarks and the
  full D1 validation (minutes each) — they run in CI.

## Amendments

**2026-08-12 (Phase 2 mandatory-items mini-round).** The update
mechanism is manual (`git pull` + `cargo build --release`) and becomes a
numbered procedure in the operations runbook in Phase 8, including
re-pinning the binsolve revision and re-running the validation batch.
Self-update was chosen first and then dropped once its collisions with
C2 (fully offline) and the private repository were worked out, so no
updater and no release workflow enter the plan: the milestone list stays
L0–L8 as approved. M30 (restore drill) joins L6, where the validation
machinery already exists.
