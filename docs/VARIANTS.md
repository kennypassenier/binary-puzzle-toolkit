# Variants: window pyramids (feat-human-1, measured 2026-11-01)

A variant here is a layout of square regions on one grid. The rule Kenny
set: **every square the layout can form is itself a binary puzzle**, so no
assembled area is left without rules.

## The rule as built: squares on the block lattice

`scripts/pyramid-geometry.py SIZE STEP MIN_SIDE` writes a geometry file in
which every square whose side is a multiple of STEP, at least MIN_SIDE and
at most SIZE, with its corner on the STEP lattice, is a region.

| Layout | Regions | Relation to the published types |
|---|---|---|
| 12, step 6, min 6 | 4 blocks + 12 | equals 4x6x6 |
| 16, step 8, min 8 | 4 blocks + 16 | equals 4x8x8 |
| 20, step 10, min 10 | 4 blocks + 20 | equals 4x10x10 |
| **18, step 6, min 6** | 9 blocks + 4 squares of 12 + 18 = 14 | 9x6x6 plus the four 12x12 squares it leaves unchecked |
| **16, step 4, min 8** | 9 blocks of 8 + 4 of 12 + 16 = 14 | new |
| **24, step 8, min 8** | 9 of 8 + 4 of 16 + 24 = 14 | new |
| **24, step 6, min 6** | 16 + 9 + 4 + 1 = 30 | new |

The published 4x6x6, 4x8x8 and 4x10x10 are the two-level pyramids, and
9x6x6 is a pyramid with its middle level missing.

## Measured

Generated with `bpt forge --geometry`, default ceiling L2 (by-hand gate):

| Layout | Puzzles | Time | Solvable by hand (`bpt report`) |
|---|---|---|---|
| 18, step 6 | 20 | 7 s | 20 of 20 |
| 20, step 10 | 20 | 50 s | 20 of 20 |
| 16, step 4 (seeds 1-3, L2) | 3 | 1 s, 55 s, 21 s | 3 of 3 |
| 24, step 8 (seeds 1-3, L2) | 2 of 3 | 52 s, 54 s, third over 60 s | 2 of 2 |
| 24, step 6 (seeds 1-3, L2) | 1 of 3 | 9 s, two over 60 s | 1 of 1 |

The denser layouts are feasible and hand-solvable; the cost is generation
time (up to about a minute per puzzle at 30 regions), not solvability.

## Not built: every square at every offset

If a 6x6 at every shift must be a puzzle, every row has to repeat every 6
cells, and then a 12x12 or 8x8 square holds two identical rows, which the
rules forbid. This is a derivation, not a measurement.
