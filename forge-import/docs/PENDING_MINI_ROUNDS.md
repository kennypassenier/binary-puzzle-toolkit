# Pending mini-rounds and deviations

Queued while Kenny was AFK (2026-08-12). Per the AFK rule in
`~/Projects/dev-procedure/PROCEDURE.md`, a deviation from a frozen
decision is never silently built: the affected area is quarantined, the
deviation is queued here, and work continues on everything unaffected.
**This queue is presented first when Kenny returns.**

## Blocking: three mini-rounds in the binsolve repository (milestone L2)

These change binsolve's frozen Phase 4 architecture, so each is a
FORM_PROTOCOL §5 mini-round in *that* project, run from a session opened
in `~/Projects/binsolve`. B1 blocks binforge L3 and everything after it.

### B1 · Caller-supplied choice oracle — blocks L3

**Original decision (binsolve AR13, frozen).** The solver core is
bit-deterministic: fixed iteration order, no randomized structures, no
time-dependent behaviour. Required by its own snapshot tests and by its
parallel-equals-sequential guarantee.

**New insight.** binforge K1 needs *random* full solutions, and its AR2
gives binsolve the branching decisions rather than duplicating them.
binsolve's `dfs` (`binsolve-core/src/search.rs:316`) already propagates,
validates the partial grid, validates on completion and snapshots each
branch with `grid.clone()`. The only decisions in it are:

```rust
let Some((row, col)) = pick_guess_cell(&grid, ctx.regions) else { … };
for value in [Cell::Zero, Cell::One] { … }
```

**The change.** Parameterise exactly those two with a caller-supplied
closure, defaulting to today's behaviour — on the order of 15 lines. The
same oracle in produces the same result out, so AR13's determinism
survives: what it forbids is randomness *inside* the solver, not a
caller choosing the order.

**Consequences.** binforge inherits binsolve's snapshotting and its
fuzzer-found fixes (`b0d0bfc`, `44798ff`) instead of shadowing them with
~90 duplicated lines, which scope C4 forbids. Nothing already built in
either project changes.

### B2 · `PuzzleKind::Custom { regions }` and a self-describing tag — blocks L8

**Original decision (binsolve AR3/AR7, frozen).** A puzzle is
`{ kind: PuzzleKind, givens: Grid }` where `PuzzleKind` is a **closed
enum** of six variants, and the tag vocabulary is the five known types.

**New insight.** Verified in `binsolve-core/src/region.rs:60`: no API
anywhere accepts an arbitrary region list. binforge K9/D4 ("two invented
types, solved end-to-end by binsolve") is therefore unbuildable as
things stand, and K4's promise "a new type needs no Rust change" is true
for binforge but false for binsolve.

**The change.** Add `PuzzleKind::Custom { regions }` plus a tag syntax
that carries the decomposition on the puzzle line itself, so an invented
type is self-describing and never needs another code change in either
tool.

**Consequences.** Must land *before* L8 development starts, not before
first emission. binforge already emits nothing until then.

### B3 · Rectangular regions — blocks L8 for band-shaped types

**Original decision (binsolve AR3, frozen).**
`Region { row, col, n, rules }` — one side length, so regions are square.

**New insight.** Scope G3 names overlapping-band compositions as a
candidate invented family, and a band is rectangular. binforge's
geometry model already accepts rectangles (L1, `ar3b_rectangular_regions_are_accepted`),
so the two models currently disagree. Blast radius measured in binsolve:
**11 sites** (`line_cells`, `search.rs:71`, `search.rs:391`,
`strategy.rs:109`, plus constructors). The rules survive unchanged:
balance checks each direction's length for evenness, unique-lines only
compares rows with rows and columns with columns, no-triples is
length-agnostic.

**Consequences.** Taken together with B2 it is one amendment instead of
two, and geometry files written before it would otherwise need
migrating.

## Non-blocking deviations, for ratification

### D-L1a · The atomic write layer was built out of order

`binforge/src/atomic.rs` belongs to L5 (batch writing) but was built
during L1, because L2 through L4 were blocked on B1 and this layer
depends on nothing. It is complete and tested (7 tests, including the
power-loss orphan-recovery trace), but nothing calls it yet, so it
carries an explicit `#![allow(dead_code)]` with a comment saying it is
wired up in L5. If L5 lands and it is still unused, that allow is the
signal something went wrong.

**Alternative if you dislike it:** delete it and rebuild it in L5.

### D-L1b · Branch protection does not stop you personally

Protection on `main` requires the three CI checks, but `enforce_admins`
is **false**, so a push from the repository owner bypasses it — the L1
push reported `Bypassed rule violations for refs/heads/main`. binsolve
is configured the same way, so the two projects match.

Setting `enforce_admins = true` would make the check binding on you too,
which in practice forces a pull-request workflow: a direct push to
`main` cannot satisfy a status check that has not run yet. That is a
real workflow change, and enforcement changes are always a gate, so it
is queued rather than applied.

### D-L1d · The size guard versus scope G1

Scope G1 says the practical size ceiling comes from the G8 performance
measurements and is **never hardcoded**. My first version of the
geometry validator contradicted that with a flat `MAX_SIZE = 32`, which
a code review caught: it would have refused a 40x40 puzzle that the
scope explicitly allows, on no evidence at all.

**What I changed instead of asking.** The constant is now
`MAX_SUPPORTED_SIZE = 256`, documented as a *resource guard against
absurd input*, not a ceiling: without any bound a file containing
`size = 4294967294` sends the renderer and the coverage scan into a loop
over billions of rows. Every plausible size (16, 20, 24, 40, 100) is
accepted and tested; how large a puzzle is *practical* stays an
answer that L7's measurements give.

**Ratify or correct:** if you would rather have no bound at all, the
renderer and the coverage scan need a different defence against a
nonsense file; if you want a lower one, it should come from measurement
in L7 rather than from me picking a number.

### D-L1e · Windows durability is weaker than Linux (for Phase 7)

`atomic::sync_dir` flushes the destination directory so a rename really
is on disk. Only Unix can open a directory to flush it, so on Windows —
a supported platform under C3 — the ordering rests on the filesystem
alone. Recorded here so it lands in `docs/TEST_PLAN.md` as a known
limitation at Phase 7 rather than being quietly assumed away. Nothing
to decide today; it becomes a Close / Accept / Later item there.

### D-L1c · Geometry feasibility is not yet detectable

K4 promises infeasible geometries are "rejected rather than looped on".
L1 validates structure only — bounds, even sides, empty regions, holes —
because proving a geometry unsolvable requires the solver (blocked on
B1). The error model already distinguishes the two answers
(`ProvedInfeasible` versus `BudgetExhausted`, tested in
`ar13_5_infeasible_and_budget_exhausted_are_different_answers`), and the
inspect command says so explicitly: *"Feasibility is not checked here."*
The check itself lands in L3 with the filler.

## Retro candidate (Phase 10)

**Classify the condition, not the error code.** The atomic write layer
tried to decide whether a failed rename was worth retrying by inspecting
the error. On Linux that logic looked right and the tests passed; on
Windows the same permanent situation (the destination is a directory)
reports a code that also means "the file is briefly locked", so a doomed
rename was retried five times and then blamed on a lock that never
existed. Windows CI caught it twice — once through `ErrorKind`, once
through the raw OS code — before the fix became "check the condition
itself up front, on every platform".

Two things this confirms for the procedure: the frozen decision to run
Windows in CI from L0 (C3) paid for itself on the very first milestone
that touched the filesystem, and a test that asserts *timing* caught a
correctness bug that assertions on the error message did not.
