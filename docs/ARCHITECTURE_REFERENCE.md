# Architecture reference

The system as built. Where a decision is worth explaining, the reason is
here; the decisions themselves are recorded in
`docs/solve/ARCHITECTURE_DECISIONS.md` and
`docs/forge/ARCHITECTURE_DECISIONS.md`.

## The shape

| Crate | Holds | Depends on |
|---|---|---|
| `bpt-core` | grid, regions, rules, the text format, strategies, the search | nothing at all |
| `bpt-forge` | geometry, filling, carving, grading, batch planning | `bpt-core`, rand, serde |
| `bpt-tui` | the replay viewer's render model | `bpt-core`, ratatui |
| `bpt` | one binary: `solve`, `forge`, `watch`, `inspect` | all of the above |

Two rules hold this apart, and CI enforces both. `bpt-core` has an empty
dependency list — not "few dependencies", empty. And neither library
contains file access, threads, printing or any clock read: all of that
lives in the binary. The clock ban is stricter than it looks and is
deliberate — a work budget must never be measured in wall-clock time,
and `Instant` is what someone would otherwise reach for.

## One mechanism for every puzzle type

There is no code path per puzzle type. A puzzle is a grid plus a list of
**regions**, and a region is a rectangle plus which of the three rules
apply to it. A plain 10x10 has one region covering everything; `4x6x6`
has five — four quadrants and the whole grid; `6in10in14` has three
nested squares. Every rule, strategy and validation walks the region
list, so a type nobody has written yet works without new code.

That is why an invented type can be a data file, and why a composite tag
can be read as a description rather than looked up.

## Solving

The solver runs a **ladder** of strategies in cost tiers, cheapest
first, to a fixpoint. If that finishes the grid, the puzzle needed no
guessing and the highest tier that fired is its difficulty. If the
ladder stalls and guessing is allowed, a depth-first search takes over,
propagating with the cheap tiers only at each node.

Three things guard against plausible wrong answers:

- A completed grid is **validated** before it is reported. Strategies
  fill cells from local rules and can complete a grid that breaks a
  global one — found by fuzzing, on the input `.10.`.
- Every branch is checked against the partial rules before it is
  explored. Without that the search walks to the bottom of branches that
  already broke a rule; adding it took an adversarial sparse 10x10 from
  2.08 seconds to 1 millisecond.
- Proving uniqueness means searching **past** the first solution. Two
  solutions found is an answer, not a failure.

## Generating

Three steps, each with a reason for being separate.

**Fill.** Complete an empty grid to a valid solution, choosing branches
from a seeded stream. The randomness lives here and never in the solver,
which stays deterministic; the generator supplies a choice oracle. The
cell chosen is the one in the most constrained line, with the randomness
in the tie-breaks — picking uniformly at random is fine on a plain grid
and pathological on the nested composites, where one 14x14 filled in
0.4 seconds and the next did not finish in ten minutes.

**Carve.** Remove clues one group at a time, keeping the puzzle uniquely
solvable and no harder than the requested level. A removal that breaks
either is put back and never retried, which converges in one pass —
carving to minimal and then repairing does not, because restoring clues
does not lower difficulty monotonically.

Uniqueness after a removal is not re-proved from scratch. Before the
removal the puzzle had exactly one solution, so it still does exactly
when pinning the removed cell to the *other* value has no solution: one
refutation instead of a full proof. That, plus dropping a redundant
search from the grading call, took a 16x16 carve from 30.8 to 5.5
seconds with byte-identical output.

**Grade.** The level is the highest ladder tier the puzzle needs, or L4
if it cannot be finished without guessing.

## Bounded work

Every uniqueness question inside a carve is bounded by a **node budget**
— search steps, never seconds. A wall clock would fire at different
points on a fast and a slow machine, and across sixteen parallel
workers, so the same seed would stop producing the same puzzle. A node
count is part of the input.

An exhausted budget is its own answer, deliberately distinct from "no
solution" and from "stuck": those describe the puzzle, this describes
the search. The carve treats it as "not proved", keeps the clue, and
counts it.

Before the budget, two of five measured 18x18 seeds never finished at
all, one still running after eighty minutes. After it, all five finish.
What it costs, measured against an effectively unbounded budget: nothing
below 16x16, and at most one extra clue where it fires.

## Batches

A batch commits completely or not at all. Puzzles are generated in
memory, then written: the per-puzzle files first, then the flat
validation file, then the directory is fsynced, then the manifest, then
fsynced again. The manifest is last on purpose — a crash can leave files
without a manifest, which reads as "no batch", but never a manifest
promising files that are not there.

Parallelism enters as a *source of candidates*, not as a change to the
logic. Which puzzles enter the batch, and in what order, is decided by a
sweep that settles indices in ascending order, so sixteen cores produce
byte-identical output to one. Candidates are speculated a chunk ahead
rather than a whole batch, because one such call cannot be interrupted
and speculating everything would mean a Ctrl-C arrived only after all
the work was done.

## The line format

One puzzle per line, `.` for empty, rows written one after another. A
composite carries a prefix that **describes its layout**: `<n>x<a>x<a>`
is n blocks of a×a, `<term>in<size>` centres that term in a larger
square, and they compose. A prefix outside that grammar is refused
rather than read as a plain grid, which would answer a different puzzle
under the same line.

Layouts whose placement is a choice rather than a consequence of a name
— overlapping regions, regions side by side — are supplied as a geometry
file to both halves instead.
