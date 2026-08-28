# Features — binforge

**FROZEN 2026-08-12** (Phase 2 gate: two rating rounds + freeze report
approved). IDs are permanent and appear in commit messages and test
names. Changes only via mini-rounds (FORM_PROTOCOL §5); amendments are
dated notes under the affected feature.

## Tally

| Rating | Count | IDs |
|---|---|---|
| Essential | 13 | K1 K2 K3a-e K4 K5 K6 K7 K8 K9 K10 K11 K12 M1 |
| Desired | 8 | M2 M3 M4 M5 M6 M7 M9 M11 |
| Later | 2 | M8 M10 |
| Don't do | 0 | — |

## Essential

### K1 · Solution filler
Complete an empty grid to a full valid solution via binsolve-core with
randomized value ordering; any even size, any geometry. Diversity
matters: repeated runs must not yield the same grid.
**Tests:** property test — filled grids satisfy all rules for every
geometry; diversity test over N runs; infeasible geometry returns a
clear error.

### K2 · Carve loop with per-step uniqueness proof
Remove clues one at a time from a K1 solution; after each removal
binsolve-core proves exactly one solution remains, else the removal is
reverted. The core algorithm.
**Tests:** every emitted puzzle independently re-proven unique; state
restored exactly after a reverted removal; end-to-end carve of a known
solution yields a solvable puzzle.

### K3 · The five special types (K3a–K3e)
Generation for the site's composite types, geometry definitions
identical to binsolve K2a–e so output is directly solvable there.
| Sub-ID | Tag | Type |
|---|---|---|
| K3a | `4x6x6` | four 6x6 quadrants forming a 12x12 |
| K3b | `4x8x8` | four 8x8 quadrants forming a 16x16 |
| K3c | `9x6x6` | nine 6x6 blocks (3x3) forming an 18x18 |
| K3d | `8in14` | 8x8 centered in a 14x14 |
| K3e | `6in10in14` | 6x6 in 10x10 in 14x14, doubly nested |
**Tests:** per type, a generated batch of ≥20 passes independent
binsolve validation; region-boundary test per type (clues in inner and
outer regions both constrain correctly).

### K4 · Data-driven geometry model
A puzzle type = grid size + list of rectangular regions, defined as
data (file format decided in Phase 4). Standard and all specials are
entries; new types need no Rust change. Invalid or infeasible
geometries are rejected with a remedy-carrying message.
**Tests:** all built-in types load and match known shapes; malformed
definitions produce remedy-carrying errors; infeasible geometry is
rejected rather than looped on.

### K5 · Difficulty measurement — four levels
Grade by solving with binsolve-core strategies; label = the ladder
level required. Levels are versioned: same level-version ⇒ same label,
reproducibly.
**Tests:** fixed reference set per level as regression anchors; level
recomputation of a sample in CI.

> **Amendment 2026-08-12 (Phase 4 mini-round).** The original wording
> defined four *technique tiers* (1 direct fills · 2 counting
> interplay · 3 unique-line eliminations · 4 deep contradiction
> chains). binsolve's `registry_stages()` puts tier 1 (`FindDuo`,
> `AvoidTriple`) and tier 2 (`FillByCount`) in the **same ladder
> stage**, and the ladder exhausts a stage before escalating, so
> `max_tier == 1` is unobservable — 0 occurrences in ~300 measured
> re-grades. The scale is redefined onto binsolve's actual ladder:
> **L1** stage 0 suffices · **L2** stage 1 needed (`KeepLineUnique` /
> `CountingArgument`) · **L3** stage 2 needed (`FillPossibilities`) ·
> **L4** guessing required. Measurable today with one
> `SolveMode::StrategiesOnly` call and no binsolve change. Nothing was
> built against the old wording. Level membership is owned by binsolve,
> so the manifest records binsolve's pinned rev and a build-time hash
> over its `(StrategyId, tier, stage)` table; a change without a
> version bump fails loudly.

### K6 · Difficulty targeting
Request type/size + tier; the carve loop steers (clue choice,
fresh-solution retries) until measured tier equals requested tier,
within the K11 time budget. An unreachable tier is reported explicitly
— never a silently mislabeled puzzle.
**Tests:** per standard size and level L1–L3, batches where every
measured level matches; L4 reachability documented per geometry;
unreachable-level path returns the explicit error.

> **Amendment 2026-08-12 (Phase 4).** "Tier" reads "level" per K5's
> amendment, and the bound is in deterministic work units, not a time
> budget (a wall-clock bound would fire at different points in parallel
> and sequential runs, falsifying M1 and M3). Targeting is achieved by
> the AR4 tier-ceiling carve — restore-and-lock a group whose removal
> pushes the grade past the target — not by post-hoc repair: measured
> evidence showed restoring clues does *not* monotonically lower
> difficulty (10x10 seed 6: +1→L3, +5→L4, +10→L3, +13→L2), so the
> original repair loop could thrash and then falsely report a level
> unreachable, violating D2.

### K7 · Dot-format output, puzzles + solutions
binsolve's G6 contract exactly: one line per puzzle, `.` for empty,
prefix tags for specials; solution emitted alongside in the same format
(line N ↔ line N).
**Tests:** format regression vectors shared with binsolve; round-trip —
a generated line parses and solves in the binsolve CLI unmodified.

### K8 · Batch generation with corpus-style layout
N puzzles per type/size/tier in one run, written into a layout
compatible with binsolve's corpus (per-type subdirectories,
informative names; naming frozen in Phase 4). Atomic writes.
**Tests:** e2e batch of 100 lands in the right structure; interrupted
run leaves no torn files (temp + rename asserted).

### K9 · Two invented types, end-to-end
At least two types binarypuzzle.com does not have (candidates
`4x10x10`, `9x8x8`, `8in12in16`; final pick when built), defined purely
as K4 geometry entries, generated and solved by binsolve after the tag
mini-round on its frozen S6b vocabulary.
**Tests:** both types generate valid unique puzzles; binsolve solves
them end-to-end; the mini-round amendment is recorded in binsolve's
docs before first emission.

### K10 · CLI
Thin binary over the library: type/size, tier, count, output location;
sensible defaults; errors carry remedies; machine-readable exit codes.
Interactive terminal may show a grid preview; pipes always get clean
dot-format.
**Tests:** e2e against the real filesystem; exit-code table covered;
help text mentions every flag.

### K11 · Benchmark harness
Encodes the scope's G8 targets (very-hard 14x14 < 10 s; 100× 10x10
< 60 s; any special < 30 s) as a repeatable measurement. Larger
invented sizes are measured and documented, not capped.
**Tests:** benchmark runs in CI (informational there); hard numbers
verified on Kenny's PC and recorded.

### K12 · Independent validation via the binsolve CLI
Test/CI harness piping generated batches through the real binsolve
binary — an independent code path — asserting exactly one solution
each. Disagreement fails the build.
**Tests:** the harness itself, plus a sabotage test (a deliberately
ambiguous puzzle must be caught).

### M1 · Seeded, reproducible generation
Every run takes an RNG seed (auto-generated if unspecified), printed
and stored with the batch; same seed + same version ⇒ byte-identical
puzzle set. The debuggability foundation for a randomized program
(standing rule 8).
**Tests:** same seed twice into an empty directory ⇒ identical output;
different seeds ⇒ different output; a failing case's
`(seed, index, attempt)` reproduces it.

> **Amendment 2026-08-12 (Phase 4, AR9).** Reproducibility is stated as
> *same seed + same version + same starting corpus*. The RNG stream is
> a function of `(batch seed, index, attempt)`; a duplicate increments
> `attempt`, so a run into a populated directory reproduces
> deterministically but differs from a run into an empty one. Every
> puzzle's triple is recorded in the manifest, so any single puzzle
> stays independently regenerable.

## Desired

### M2 · Duplicate detection
Hash emitted puzzles; refuse duplicates within the batch and against
the target corpus. Small grids have a finite space and silent repeats
weaken a test corpus.
**Tests:** no duplicate lines in a batch; re-running into an existing
corpus adds only new puzzles; collisions reported, not silently
skipped; re-rolls resolved in index order so parallel equals
sequential.

> **Amendment 2026-08-12 (Phase 4, AR9).** A collision is resolved by a
> **deterministic re-roll** (attempt counter in the RNG stream), not by
> refusing the write: under AR10's all-or-nothing batches a refusal
> would fail the whole batch. Collisions must resolve in ascending
> index order — completion order would make the outcome depend on which
> rayon worker finished first.

### M3 · Parallel generation
Generate independent puzzles across CPU cores, with deterministic
per-worker seed derivation so M1 reproducibility survives.
**Tests:** parallel and sequential runs with the same seed produce
identical sets; scaling measured in K11.

### M4 · Batch manifest with statistics
Sidecar manifest per batch: seed, tool version, requested
geometry/tier, per-puzzle measured tier and clue count, generation
time, tier-version used for grading.
**Tests:** manifest matches emitted files exactly (count, hashes);
remains valid and parseable after an interrupted run.

### M5 · Clue-count control and symmetry options
Target clue count / carve-as-far-as-possible mode, plus optional
symmetric clue patterns (rotational or mirror). Symmetry constrains the
carve and costs generation time.
*(Upgraded from Claude's "Later" recommendation to Desired by Kenny at
the round-2 gate.)*
**Tests:** requested clue counts respected or explicitly reported
unreachable; symmetric output verified symmetric under the chosen
transform.

### M6 · Geometry inspect / validate command
CLI subcommand rendering a geometry as ASCII art (cell → region
membership) plus a feasibility verdict, without generating. The
debugging tool for K9.
**Tests:** ASCII rendering asserted per built-in type; a broken
geometry gets a diagnosis naming the offending region.

### M7 · Progress reporting and cancellation
Progress on interactive terminals (n/N, elapsed, current geometry),
silence when piped, and Ctrl-C finishing the current puzzle and
writing what is done.
**Tests:** piped output free of progress noise; simulated cancellation
leaves a complete, valid partial batch plus a manifest with
`status: cancelled`.

> **Amendment 2026-08-12 (Phase 4, AR10b).** Batches are otherwise
> all-or-nothing: errors and crashes discard the batch. A deliberate
> Ctrl-C is the single exception and is marked `status: cancelled` with
> a non-zero exit, so a long run's work survives while no consumer can
> mistake it for a complete batch. Responsiveness depends on binsolve
> mini-round B4 (node budget) — without it, cancellation waits for the
> current uniqueness proof, worst measured 15.9 s.

### M9 · Fuzz / property testing of the pipeline
Property tests over random geometries, sizes and seeds asserting the
always-invariants: emitted puzzles uniquely solvable; puzzle is a
subset of its solution; measured tier reproducible; no run hangs
forever.
**Tests:** it is the test; failures shrink to a minimal reproducing
seed + geometry (replayable via M1).

### M11 · Restore drill
Clone the repository fresh into an empty directory, take a manifest
from a previous batch, regenerate from its recorded seed triple, and
diff against the original corpus byte for byte. Proves the backup
restores *and* proves the corpora were genuinely regenerable rather
than merely claimed to be; fails loudly if reproducibility ever breaks
silently. Executed in L6 where the validation harness already exists,
and written up as a numbered restore procedure in the operations
runbook (Phase 8).
**Tests:** the drill itself, run against a fixture batch in CI; a
deliberately altered manifest makes it fail.

> **Added 2026-08-12 (Phase 2 mandatory-items mini-round, V3b).**

## Later

### M8 · Printable output (PDF or HTML sheets)
Printable grids — e.g. six 10x10 puzzles per sheet with solutions
overleaf. Parked until the intended use is clear; commits to a
rendering stack.

### M10 · Difficulty distribution report
Reads an existing corpus and reports tier distribution, clue-count
spread and dominant strategies per tier. Meaningful only once corpora
exist and tier definitions have settled.

## Decision log

| Date | Decision |
|---|---|
| 2026-08-12 | Round 1 (K1–K12, scope-derived): all Essential |
| 2026-08-12 | Round 2 (M1–M10, Claude's proposals): M1 Essential; M2–M7, M9 Desired (M5 upgraded from Later by Kenny); M8, M10 Later |
| 2026-08-12 | List frozen (freeze report R1–R3 approved) |
| 2026-08-12 | Phase 4 mini-rounds: K5/K6 difficulty scale redefined onto binsolve's ladder (L1–L4); M1 reproducibility conditioned on the starting corpus; M2 collisions resolved by deterministic re-roll; M7 cancellation as the exception to all-or-nothing batches |
| 2026-08-12 | K4/K9 acknowledged as blocked on binsolve mini-rounds B2 (custom geometry) and B3 (rectangular regions) — "no Rust change" holds for binforge, not for binsolve, until B2 lands |
| 2026-08-12 | Phase 2 mandatory-items mini-round: update mechanism = **manual, documented in the runbook** (self-update chosen at V1, then dropped once its collisions with C2/offline and the private repository were worked out — no K13, no release workflow); ecosystem = binsolve contract recorded in ECOSYSTEM.md, no integration with latch/mailbox/homelab (V2a/V2b); backup = state-in-git + private GitHub remote, manual push, restore drill M11 in L6 (V3a/V3b) |
