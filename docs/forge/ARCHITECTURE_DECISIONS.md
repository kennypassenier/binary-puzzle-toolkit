# Architecture Decisions — binforge

Phase 3 entries decided 2026-08-12 (tech-choice gate, all items as
recommended). Phase 4 entries follow. Changes only via mini-rounds
(FORM_PROTOCOL §5).

## Phase 3 · Tech choice

| ID | Decision | Choice |
|---|---|---|
| T1 | Toolchain, edition, MSRV | rustup + `rust-toolchain.toml` pin; edition 2024; no MSRV (track stable) — identical to binsolve T1/T2/T12 |
| T2 | Workspace layout | two crates: `binforge-core` (lib, zero ambient I/O) + `binforge` (CLI: clap + anyhow) |
| T3 | binsolve-core dependency | git dependency pinned to a commit (`https://github.com/kennypassenier/binary-puzzle-toolkit.git`, `rev = …`); local path override allowed for co-development |
| T4 | RNG | `rand` + `rand_chacha` (`ChaCha8Rng`): reproducible across versions/platforms, u64-seedable, per-worker stream derivation for M22 |
| T5 | Geometry definitions (K23) | TOML via serde — comment-friendly, hand-editable |
| T6 | Manifest format (M23) | JSON via serde_json — program-written, `jq`-greppable, feeds M29 |
| T7 | Parallelism (M22) | rayon — input-order collection keeps parallel-equals-sequential honest |
| T8 | Duplicate detection (M21) | `HashSet<String>` of puzzle lines, zero deps — exact, no collision reasoning |
| T9 | Progress + cancellation (M26) | hand-rolled stderr progress line (silent when not a TTY) + `ctrlc` crate for portable Linux/Windows signal handling |
| T10 | Testing / benchmarking | proptest (M28, shrinking), insta (M25 ASCII + CLI help snapshots), criterion (K30) + a release-mode test asserting hard G8 thresholds |
| T11 | Dependency policy | strict allowlist; new dependency ⇒ mini-round. Runtime: binsolve-core, clap, thiserror, anyhow, rand, rand_chacha, serde, toml, serde_json, rayon, ctrlc. Dev: proptest, insta, criterion. (Zero-dep core is not achievable here — RNG and geometry parsing live in core — so the allowlist is the discipline.) |
| T12 | License | MIT — matches binsolve |

## Phase 4 · Architecture

**FROZEN 2026-08-12** (architecture gate + one deep-dive round on AR21,
AR22b, AR25, AR28). Draft attacked by the `architecture-critic` agent —
41 objections, 14 blocking; the agent built a probe crate against the
real `binsolve-core` and measured on Kenny's PC. Structural claims
independently verified against binsolve HEAD `44798ff` before being
presented. Changes only via mini-rounds.

### Queued binsolve mini-rounds (blocking dependencies)

| # | Change to binsolve | Needed for | Blocks |
|---|---|---|---|
| B1 | Choice oracle: parameterise `pick_guess_cell` + value order with a caller-supplied closure (~15 lines; defaults preserve today's behaviour) | AR21 | K20 |
| B2 | `PuzzleKind::Custom { regions }` + self-describing tag syntax | AR22 | K28, D4 |
| B3 | Rectangular regions (`Region` gains a second side length; 11 touch points) | AR22b | K28 with bands |
| B4 | Deterministic node budget → `SolveOutcome::BudgetExhausted` | AR26 | AR29, M26 |
| B5 | Git rev in `--version` | AR27 | K31 skew detection |

### AR20 · Crate layout and purity
`binforge-core` (library: geometry model, solution filling, carve loop,
grading, batch planning) with zero ambient I/O and
`#![forbid(unsafe_code)]`; `binforge` (binary: clap CLI, file writing,
progress, signals, rayon pool). CI asserts core contains no `std::fs`,
`println!`, `std::thread` **and no `std::time` at all** — `Instant`,
not just `SystemTime`, is what a time budget would reach for (AR32.3).
Geometry TOML parsing lives in core (string → model); file reading in
the CLI.

### AR21 · Randomness via a caller-supplied choice oracle (B1)
binsolve's DFS (`search.rs:316`) already propagates, validates the
partial grid, validates on completion and snapshots per branch
(`grid.clone()`); the only decisions are `pick_guess_cell` and the
fixed `[Zero, One]` value order. B1 parameterises exactly those two
with a caller-supplied closure, defaulting to current behaviour —
~15 lines. binforge owns the RNG; binsolve stays bit-deterministic
(same oracle in ⇒ same result out), and binforge inherits its
snapshotting and its fuzzer-found fixes (`b0d0bfc`, `44798ff`)
instead of shadowing them with ~90 duplicated lines (which scope C4
forbids).

### AR22 · Invented types: self-describing custom tag
binsolve's `PuzzleKind` is a **closed enum** (`region.rs:60`) with no
API accepting an arbitrary region list, so K28/D4 are unbuildable as-is
and K23's "no Rust change" is false today. Decision: a binsolve
mini-round adds `PuzzleKind::Custom { regions }` plus a tag syntax
carrying the decomposition on the puzzle line, so an invented type is
self-describing and needs no further code change in either tool. This
amends binsolve's **AR22 (geometry model)**, not merely its AR26 tag
vocabulary, and must land **before K28 development starts**.

### AR22b · Rectangular regions (B3)
`Region` gains a second side length. Blast radius measured: 11 sites
(`line_cells`, `search.rs:71`, `search.rs:391`, `strategy.rs:109`,
constructors). Rules survive: balance checks each direction's line
length for evenness, unique-lines only ever compares rows with rows and
columns with columns, no-triples is length-agnostic. Taken now, in the
same amendment as B2, because the geometry TOML is user-facing data —
adding a side length later means migrating written geometry files —
and because scope G3's overlapping-band family is the one genuinely
novel geometry in the project.

### AR23 · Carve loop: tier-ceiling invariant
Remove a group → prove uniqueness → grade → **if the grade exceeds the
target level, restore and lock the group**. Produces "minimal subject
to the requested level", converges in one pass, never restarts.
Rejected: carve-to-locally-minimal then repair — measured, 23 of 28
minimal puzzles (8 of 8 at 14x14) were unsolvable by any tier without
guessing, i.e. outside the difficulty scale entirely; and restoring
clues does **not** monotonically lower difficulty (measured: 10x10
seed 6 went +1→t3, +5→t4, +10→t3, +13→t2), so the repair loop can
thrash and then falsely report a level unreachable (a D2 violation).

### AR24 · Difficulty scale = binsolve's ladder stages
*(Mini-round on frozen K24/K25, 2026-08-12.)* binsolve's
`registry_stages()` puts tier 1 (`FindDuo`, `AvoidTriple`) and tier 2
(`FillByCount`) in the **same stage**, and the ladder exhausts a stage
before escalating, so `max_tier == 1` is unobservable (0 occurrences in
~300 measured re-grades) and K25's tier-1 test could never pass. New
scale, measurable today with one `SolveMode::StrategiesOnly` call and
no binsolve change:
| Level | Meaning |
|---|---|
| L1 | ladder stage 0 suffices |
| L2 | stage 1 needed (`KeepLineUnique` / `CountingArgument`) |
| L3 | stage 2 needed (`FillPossibilities`) |
| L4 | guessing required |
K24/K25 receive dated amendments in FEATURES.md.

### AR25 · Performance: mechanism frozen, numbers calibrated at the milestone
The alarming measurements (`4x8x8` at 41.3 s against scope G8's 30 s
ceiling; 120× seed spread; 20x20 unfinished after 2 minutes) were taken
on carve-to-locally-minimal — the strategy AR23 rejects. Near-minimal
grids are exactly where uniqueness proofs explode; AR23's tier-ceiling
carve keeps more clues, which makes every proof cheaper by an unknown
but likely large factor. Freezing numbers now would freeze them
against an algorithm we are not building.

Frozen mechanism: per (geometry, level), record median and p95 over 20
seeds into a baseline file; CI fails when p95 regresses beyond 1.5× the
recorded baseline; every user request additionally carries a work-unit
ceiling so nothing runs unbounded. The numbers are measured and
recorded as a milestone exit criterion once the carve loop exists.
Scope G8/D3's original figures are amended to provisional.

### AR26 · Bounding in deterministic work units
All loops bound by work units (uniqueness proofs, restarts, node
budget) — **never wall-clock**, which under 47× seed variance and 16
rayon workers would fire at different points in parallel and
sequential runs and thereby falsify M20 and M22. A binsolve mini-round
adds a deterministic node budget (`SolveOutcome::BudgetExhausted`):
without it the finest interruptible unit is one `solve()` call, worst
measured 15.9 s, so AR29's "nothing hangs" guarantee and M26's
responsive Ctrl-C are undeliverable.

### AR27 · Validation: two artifacts, pinned binary
`--check` (verified, `bpt/src/main.rs:217`) validates a *supplied*
solution and never proves uniqueness — K31's sabotage test would pass
vacuously against it. Each batch therefore emits **two** artifacts: the
two-line corpus files (archival + `--check` consistency) and a flat
one-puzzle-per-line file for `binsolve --file … --unique`, whose exit
code and absence of `#multiple:` markers is the real K31 assertion.
The validating binary is **built from the pinned rev**, never taken
from `PATH`, and its rev is recorded in every manifest; a binsolve
mini-round puts the git rev into `--version` (today: `0.1.0` forever).

**Amendment 2026-08-29 (L6, implementation).** The merge dissolved the
pinning problem rather than solving it. There is no second repository to
pin: the harness runs the binary Cargo just built for the test
(`CARGO_BIN_EXE_bpt`), so generator and validator are the same build by
construction and version skew cannot occur. What the decision was
protecting — never validating against an unknown binary from `PATH` —
holds more strongly than the original mechanism could deliver.

The independence the decision asks for is unaffected: validation goes
through `bpt solve --file --unique`, the solver's search, which shares
no code with fill or carve. B5 is built, so every manifest still records
the revision that produced the batch.

### AR28 · Duplicates: deterministic re-roll on an attempt counter
The RNG stream for a puzzle is a function of **(batch seed, index,
attempt)**. A collision — within the batch or against the existing
corpus — increments `attempt` and regenerates from a fresh, fully
determined stream, so batches stay full *and* reproducible. Two
requirements make it hold:
1. Collisions resolve in **index order**, never completion order:
   generate all at attempt 0 in parallel, then sweep indices
   ascending; any index clashing with a lower index or with the
   pre-existing corpus re-rolls at attempt+1; repeat until stable.
   Otherwise M22's parallel-equals-sequential test fails.
2. Reproducibility is conditional on the starting corpus. Same seed
   into an empty directory always reproduces the batch; into a
   populated one it reproduces deterministically but differently. Every
   puzzle's `(seed, index, attempt)` is recorded in the manifest, so a
   single puzzle is always regenerable for debugging.

Kenny's proposal at the deep-dive round; it also fits AR29 better than
refusing writes would, since under all-or-nothing a refused duplicate
would fail the entire batch.

### AR29 · All-or-nothing batches
A batch commits completely or not at all; a run owns its output
directory and refuses a non-empty one without an explicit flag.
Manifest carries `requested`, `completed`, `status`. Durability: fsync
the destination **directory** after the final rename, then write the
manifest, then fsync again — without the directory fsync the
manifest-last ordering is not actually crash-safe (a crash can leave a
manifest claiming 100 puzzles with three files missing).

**AR29b · Cancellation is the single exception.** Errors and crashes
discard the batch (all-or-nothing holds); a deliberate Ctrl-C writes
what is finished with `status: cancelled` and a non-zero exit, so an
hour of compute on a large batch is never thrown away and no consumer
can mistake the result for a complete batch. M26 keeps its
partial-batch promise under that status.

### AR30 · Naming, exit codes, corpus location
Files: `bf-<seed>-<index>-<level>.txt` — every file names what
reproduces it. Exit codes: 0 all requested produced · 1 partial (level
unreachable, budget exhausted, duplicates refused) · 2 usage / file /
geometry error.

**Amendment 2026-08-29 (L5, implementation).** "Level unreachable" can
no longer occur: AR23 makes the ceiling an invariant of the carve loop,
so a carve never returns a puzzle above the requested level and there is
nothing to report short. The implemented exit-1 causes are duplicates
refused, and budget exhausted once AR26's budget exists. The `--level`
flag therefore behaves as a target in practice as well as a ceiling —
measured over 40 seeds on a 10x10: ceiling L1 produced 40 L1 puzzles,
L2 produced 40 L2, L3 produced 16 L3 and 24 L2, L4 produced 31 L4, 1 L3
and 8 L2. Generated corpora live under binforge's own root and
are **never** written into binsolve's `corpus/`: binsolve's calibration
test derives site labels by parsing filename suffixes (a `…tier4.txt`
would read as *easy*) and its threshold test loads every `.txt` there
and asserts worst-case < 1 s, which generated near-minimal puzzles
would break.

### AR31 · Carve operates on removal groups
The loop always removes a *group* of cells; a group of size 1 is the
ordinary case. Makes M24's symmetric carving (orbits of 2 or 4)
additive instead of a rewrite of the central loop.

### AR32 · Adopted correctness details
1. binsolve's `run_to_fixpoint` reports `Solved` **without validating**
   (`search.rs:48`) — binforge calls `validate_solution` on every
   completion and `validate_partial` after every branch assignment.
2. On a refuted branch, restore the pre-assignment snapshot before
   trying the other value (flipping in place leaves deductions derived
   from the refuted value behind — silent corruption, no crash).
3. Core bans the whole `std::time` module (see AR20).
4. Geometry TOML exposes binsolve's three rule toggles (balance,
   no-triples, unique-lines), default all-on.
5. Error model distinguishes "proved infeasible" from "gave up after N
   work units" — K23's test needs them to be different answers.
6. CI clones binsolve at the pinned rev and diffs the shared format
   fixtures; a git dependency does not expose the dependency's
   `tests/` directory, so "copied and asserted identical" is otherwise
   decoration. Fixtures pinned `-text` in `.gitattributes`.
7. Subprocess validation gets a kill timeout. Solution-distribution
   bias from row-major branching is explicitly accepted as harmless
   for a corpus (recorded, not silently ignored).
