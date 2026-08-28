# Architecture reference — the system as built

`docs/ARCHITECTURE_DECISIONS.md` records what was decided and why, with
the counter-arguments that survived the critic. This document describes
what actually exists, including where the build diverged from the plan.

## The shape

Three crates in one cargo workspace:

```
binsolve-core   pure solving logic, zero runtime dependencies
binsolve        the CLI, a thin frontend over the core
binsolve-tui    the terminal replay viewer, a second frontend
```

The core has no I/O of its own: it does not print, read files, or know
what a terminal is. That is what makes every rule testable in isolation
and why both frontends can sit on top without duplicating logic. CI
asserts the core's `[dependencies]` section stays empty (AR1, T10).

## How a puzzle is represented

A puzzle is **one grid plus a list of regions**. The grid is a flat
row-major `Vec<Cell>` where `Cell` is `Zero`, `One` or `Empty` — one
byte per cell, so the largest board (18×18) is 324 bytes and copying a
grid for a search branch is a cheap memcpy.

A region is a square area `{row, col, n, rules}`. Every rule is checked
per region, never against the raw grid. That single mechanism covers
all six puzzle kinds:

| Kind | Regions |
|---|---|
| standard n×n | 1 (the whole grid) |
| `4x6x6` | 4 blocks + the whole 12×12 |
| `4x8x8` | 4 blocks + the whole 16×16 |
| `9x6x6` | 9 blocks + the whole 18×18 |
| `8in14` | the centred 8×8 + the whole 14×14 |
| `6in10in14` | the centred 6×6, the 10×10 around it, the whole 14×14 |

The consequence worth internalising: cells 5 and 6 of a row in a tiled
puzzle are *neighbours* in the whole-grid region but sit in *different*
blocks. A no-triple rule across that seam exists only at the whole-grid
level, while a count rule can fire inside one block while the wide row
stays silent. Both directions are pinned by tests in
`binsolve-core/tests/specials.rs`.

`RuleSet` allows a region to enforce a subset of the rules. Today every
region enables all three; the field exists so that a future puzzle type
that relaxes one is a data change rather than a rewrite.

## How solving works

Two engines, in order.

**1 · The strategy ladder.** Six named strategies run over every line of
every region until nothing new is deduced. They are grouped in tiers by
cost, and a cheaper tier is exhausted before a costlier one runs; any
progress drops back to the cheapest tier.

| Tier | Strategy | What it sees |
|---|---|---|
| 1 | FindDuo | two equal neighbours force the cells beside them |
| 1 | AvoidTriple | `0.0` forces the middle to `1` |
| 2 | FillByCount | a line that holds all its `0`s forces the rest to `1` |
| 3 | KeepLineUnique | a nearly-complete line may not duplicate a finished one |
| 3 | CountingArgument | a value is refused when no completion of the line survives it |
| 4 | FillPossibilities | enumerate every legal completion; cells all of them agree on are forced |

Every deduction carries its cell, its value, the strategy, and a
*structured* reason — not a formatted string. The text is rendered only
when someone asks for it, so the search loop never allocates messages
nobody reads.

**2 · Depth-first search.** When the ladder stalls, the solver picks the
line with the fewest empty cells, guesses its first empty cell, and
propagates. Only tier 1–2 strategies run inside the search, which keeps
the cost of a node bounded — the reason the worst-case time promise is
possible at all.

Two additions the build discovered, both from failures rather than from
the plan:

- After the ladder completes a grid, the result is **validated** before
  being reported. "No empty cells" is not the same as "correct": the
  strategies fill cells from local rules and can complete a grid that
  breaks a global one. The fuzzer found this with `.10.`, a 2×2 whose
  forced filling makes both rows identical.
- At every search node the **partial grid is validated**. Strategies act
  only on empty cells, so a branch that had already over-filled a line
  was invisible to them and got explored to the bottom. Adding this cut
  a sparse 10×10 from 2.08 s to 1 ms.

## Outcomes

```
Solved { solution, stats }          one grid, with the work it took
MultipleSolutions { first, second } the puzzle is ambiguous
Contradiction { reason }            no solution, and why in plain words
Stuck { grid, filled }              strategies-only mode ran out of ideas
```

`Contradiction` never says just "failed": it names the violated rule and
its location, or reports that the search refuted every assignment.

Three solve modes select how far the engine goes: strategies only, stop
at the first solution, or continue past it to prove the solution unique.
That last mode is what makes a scraped puzzle self-validating — exactly
one solution means the answer is verified without a published one.

## The event stream

One observer interface carries everything the outside world learns about
a solve: `Deduced`, `Guessed`, `Backtracked`, `SolutionFound`. Four
consumers read the same stream — the `--explain` trace, the terminal
statistics, difficulty grading, and the TUI. When nobody observes, the
path costs nothing.

Statistics count only the work up to the first solution. Proving
uniqueness means refuting every alternative branch, which would
otherwise make every uniqueness-checked puzzle look extremely hard.

The TUI does not watch a live solve. It solves at full speed into a
recorded log and replays that log at whatever pace you choose, so the
timing statistics stay honest and rendering is deterministic. Undoing a
guess restores the entire frame from before it, because the refuted
branch also deduced cells — an earlier version cleared only the guessed
cell and showed states the solver never held.

## Determinism

The core is bit-deterministic: fixed iteration order over regions, lines
and strategies; no hash-based containers in solver paths; no dependence
on time. Three things rest on it — the pinned trace fixture, trace
replay, and the premise that a parallel batch would equal a sequential
one. `binsolve-core/tests/determinism.rs` asserts it rather than
assuming it.

## Text format

One puzzle is one line: the grid row by row, `.` for empty, optionally
prefixed with a type tag and a colon.

```
1..0.0..1...0..1.1....0....1.0..11..      a 6x6, size inferred from length
4x8x8:110..0.1…                            a composite, 256 cells
```

Output mirrors the input with the dots filled in, so line N of the
output always describes line N of the input. A puzzle that cannot be
solved keeps its slot and its original text behind a marker
(`#contradiction:`, `#multiple:`, `#stuck:`, `#invalid:`), which is why
a blank line is reported rather than skipped — dropping it would shift
every later line onto the wrong puzzle.

The grammar is pinned as regression vectors in
`binsolve-core/tests/fixtures/format/`, and those files are marked
`-text` in `.gitattributes` so no platform can rewrite their line
endings.

## File writing

Results are written atomically: a temporary file in the destination
directory, flushed and synced, then renamed over the target. A power cut
therefore leaves either the old complete file or the new one, never a
half-written result. On Windows a rename can fail because another
process holds the file; the write retries briefly before reporting an
error that names the file.

Two honest notes: the retry currently reacts to any I/O error rather
than specifically to a sharing violation, and the cleanup of an orphaned
temporary file promised in the decision record does not exist yet. Both
are open items in `docs/TEST_PLAN.md`.

## What is deliberately absent

- **No network code, no credentials, no telemetry.** The dependency
  allowlist is enforced in CI, so nothing can quietly acquire an HTTP
  client.
- **No parallelism.** It was designed but not built: 1,000 puzzles take
  under 0.2 s single-threaded against a 30 s budget, so threading would
  add a dependency and ordering complexity to save nothing measurable.
- **No puzzle generation.** binsolve solves and validates; generating is
  its companion project's job.
