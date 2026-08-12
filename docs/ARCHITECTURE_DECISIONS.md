# Architecture Decisions — binsolve

Phase 3 entries decided 2026-08-12 (tech-choice gate, all items as
recommended). Phase 4 entries **FROZEN 2026-08-12** (architecture gate
+ deep-dive round on AR2/AR5; draft attacked by the
architecture-critic agent — 15 objections, all resolved by adoption or
by Kenny's decision below). Changes only via mini-rounds.

## Phase 3 · Tech choice

| ID | Decision | Choice |
|---|---|---|
| T1 | Toolchain management | rustup + `rust-toolchain.toml` pin (nightly available for T9) |
| T2 | Rust edition | 2024 |
| T3 | CLI parsing | clap (derive) |
| T4 | TUI stack | ratatui + crossterm |
| T5 | Error handling | thiserror (library), anyhow (binaries) |
| T6 | Property testing | proptest (dev-dep) |
| T7 | Snapshot testing | insta (dev-dep) |
| T8 | Benchmarking | criterion (dev-dep); hard G5 thresholds also in a release-mode test |
| T9 | Fuzzing | cargo-fuzz / libFuzzer, nightly, Linux, on-demand |
| T10 | Dependency policy | strict allowlist: runtime clap, ratatui, crossterm, thiserror, anyhow (+rayon reserved for M5); dev proptest, insta, criterion, cargo-fuzz. **Solver core: zero dependencies (pure std).** New dep ⇒ mini-round |
| T11 | License | MIT |
| T12 | MSRV | none — track current stable via the T1 pin |

**T5 amendment (2026-08-12, AR6 gate item):** thiserror is used in the
CLI/TUI binaries only. `binsolve-core` hand-writes its Display/Error
impls to keep the zero-dependency rule (T10) intact.

## Phase 4 · Architecture (frozen)

### AR1 · Workspace: three crates
`binsolve-core` (lib, zero runtime deps, `#![forbid(unsafe_code)]`),
`binsolve` (CLI: clap + anyhow), `binsolve-tui` (K15: ratatui +
crossterm). CI asserts core's `[dependencies]` is empty
(dev-dependencies allowed).

### AR2 · Grid = flat `Vec<Cell>`, `Cell = {Zero, One, Empty}`
Row-major, index (r,c) = r·n+c; max grid 324 bytes; branch copy =
cheap memcpy. No bitboards, no candidate masks — at these sizes the
bottleneck is search-tree shape, never cell access; readable rule code
feeds K16 traces. (Deep-dive decision.)

### AR3 · Composite puzzles = overlapping constraint regions, with rule mask
Puzzle = grid + `Vec<Region>`; Region = {origin, size, rules:
RuleSet}. Plain n×n → 1 region; `4x8x8` → 5; `6in10in14` → 3
concentric. Strategies/validation operate only on lines of a region.
RuleSet is all-on for every known tag today — empirically verified
2026-08-12 on one published solution per special type (all rules hold
at all levels, incl. whole-grid uniqueness on tiled types); the mask
exists so a future counter-example is a data fix, not a rework. M3
`--check` over the full corpus guards the assumption.

### AR4 · Strategy trait with lazy, structured attribution
Strategies: pure `LineContext → Vec<Deduction>`; Deduction = {cell,
value, strategy, structured reason}. Reasons render to text only when
an observer asks (never eager Strings in the search loop). Tier number
feeds M2.

### AR5 · Search: fixpoint, then DFS; cheap tiers propagate in-search
Top level: full strategy registry to fixpoint. Stuck → DFS on the line
with fewest empty cells (>0), first on ties; inside DFS only tier 1–2
(constant-cost) strategies propagate; tiers 3–4 run between search
episodes. Uniqueness (K5): search continues to a second solution. M1 =
same loop, DFS disabled. (Deep-dive decision; keeps node cost bounded
for the G5 worst-case promise and keeps M2's guess count meaningful.)

### AR6 · Outcomes and errors
`SolveOutcome = Solved{solution, stats} | MultipleSolutions{first,
second} | Contradiction{reason} | Stuck{filled}`. ParseError variants
carry remedy text (K7). Core never panics on any input (M7 fuzz target
asserts). Errors hand-rolled in core per the T5 amendment.

### AR7 · Text format grammar (frozen with regression vectors)
Line := `[tag ":"] grid`; tags `4x6x6 4x8x8 9x6x6 8in14 6in10in14`;
grid chars `0 1 .`; untagged length must be a perfect even square.
Failure lines: `#contradiction:` `#multiple:` `#stuck:` `#invalid:` +
original line. Exit codes: 0 all solved · 1 ≥1 failed · 2 usage/file
error. Input tolerates CRLF and missing final newline; fixtures pinned
`-text` in .gitattributes. Regression vectors in
tests/fixtures/format/.

### AR8 · Single event stream
Observer events: Deduced, Guessed, Backtracked, SolutionFound, Done.
Consumers: K16 --explain (stderr/file), K11 stats, M2 grading, K15
TUI. Stats/grading count only pre-first-solution work. No-observer
path ~zero cost (asserted in K13 benches).

### AR9 · TUI = record-then-replay
Solve at full speed into an in-memory event log; TUI replays at any
speed. Honest timing stats, deterministic rendering, testable render
model, no solver/UI threading.

### AR10 · Concurrency at puzzle granularity
Core single-threaded per puzzle; M5 (if activated) parallelizes the
batch, output reassembled in input order. `--explain` forces
sequential execution.

### AR11 · Atomic writes
Temp file in destination dir → flush+sync → rename over destination
(std rename replaces on Windows via MOVEFILE_REPLACE_EXISTING). On
Windows sharing violation: bounded retry with backoff, then fail with
remedy. Power loss at any moment leaves old-complete or new-complete
file, plus at most an orphan .tmp cleaned by the next run.

### AR12 · Corpus format and naming
Corpus file: line 1 = puzzle in AR7 line format; line 2 (optional) =
`solution:<grid>`. This two-line format is corpus/`--check`-specific,
NOT part of the AR7 user grammar. Names:
`corpus/{standard|special}/<size-or-tag>/bp-<puzzleid>-<date>-<difficulty>.txt`.
M4 meta-test parses line 1 and verifies line 2 via M3 logic.

### AR13 · Determinism invariant
Core is bit-deterministic: fixed region/line/strategy iteration order;
no Hash* or randomized structures in solver paths; no time-dependent
behaviour. Required by K16 snapshots, trace-replay property test, M5
parallel-equals-sequential.
