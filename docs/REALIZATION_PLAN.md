# Realization Plan — binforge

Phase 5 outcome, approved by Kenny 2026-08-12 (all milestones agreed
unchanged, standing rules confirmed, hook configuration as proposed,
AFK mode off). Enforcement was installed before L0 per the procedure.

## Status

| Milestone | Features | Status |
|---|---|---|
| L0 · Walking skeleton, CI green | [meta] | not started |
| L1 · Geometry model + inspect | K4, M6, AR3b, AR13.4 | not started |
| L2 · binsolve mini-rounds B1–B3 | external (binsolve repo) | not started |
| L3 · Random solution filler | K1, M1, AR2, AR9, AR13.1 | not started |
| L4 · Carve loop + grading + targeting | K2, K5, K6, AR4, AR12 | not started |
| L5 · Output, batches, CLI | K7, K8, K10, M2, M4, M7, AR8, AR10, AR11 | not started |
| L6 · Independent validation harness | K12, D1, AR8, B5 | not started |
| L7 · Parallelism, benchmarks, baselines | M3, K11, AR6, B4 | not started |
| L8 · Invented types + property testing | K9, D4, M9, M5 | not started |

Remaining after L8: K3 (the five special types) is covered
incrementally — its geometries land in L1, its generation in L3/L4, its
validation batches in L6.

## Ordering rationale

- **L1 before L2** — the geometry model needs nothing from binsolve, so
  it proceeds while the blocking mini-rounds are pending instead of
  idling behind them, and M6's inspect command is the debugging tool
  for every milestone after it.
- **L2 is external.** B1 (choice oracle) blocks K1 and therefore
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
malformed geometry names the offending region; K4's
error-vs-infeasible distinction covered by tests.

### L2
The three amendments recorded in binsolve's docs with its own suite
green; binforge pins the resulting rev.

### L3
Filled grids valid for every built-in geometry (property test); same
seed ⇒ byte-identical, different seeds ⇒ different; every completion
independently validated (AR13.1); infeasible geometry rejected within
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
puzzle is caught; a mismatched binary is detected, never silently used.

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
- `.claude/hooks/core-purity.sh` — AR1: no `std::fs`, `std::thread`,
  `std::time`, or print macros in `binforge-core`.
- `.github/workflows/ci.yml` — the same gates on Linux and Windows,
  plus the purity job. Red blocks merge.
- Deliberately **not** blocking commits: criterion benchmarks and the
  full D1 validation (minutes each) — they run in CI.
