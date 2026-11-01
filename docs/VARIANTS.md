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

Generated with `bpt forge --geometry --level L2` (the by-hand gate), one
seed per run with a 150 s cap, 12 runs at a time on 16 cores. All
`bpt report --geometry` verdicts: every puzzle L2, every puzzle solvable by
hand.

| Layout (size, step, smallest) | Regions | Seeds that finished | Puzzles solvable by hand |
|---|---|---|---|
| 18, 6, 6 | 14 | 60 of 60 (7 s and 26 s per 30) | 60 of 60 |
| 12, 4, 8 | 5 | 40 of 40 | 40 of 40 |
| 16, 4, 8 | 14 | 15 of 40 | 15 of 15 |
| 24, 8, 8 | 14 | 19 of 40 | 19 of 19 |
| 24, 6, 6 | 30 | 21 of 40 | 21 of 21 |
| 20, 10, 10 | 5 | 40 of 40 (equals 4x10x10) | 40 of 40 |

195 puzzles in total, 195 solvable by hand. Ten of each new layout are
kept in `variants/` with their geometry in `geometries/`, and
`feat_human_1_every_variant_puzzle_is_solvable_by_hand` re-checks them.

The dense layouts (14 and 30 regions) finish about half of the seeds in
150 s; the rest are slow, not impossible. At level L1 the 16x16 pyramid
did not finish a batch of 30 in 49 minutes, so these layouts are L2-only
in practice.

## Not resolved

- Step 2 lattices (14, 2, 6 and 16, 2, 8: 55 regions) and 20, 4, 12 (14
  regions): no solution found in 200-300 s, including with almost no
  carving. Feasibility is open, not disproved.
- Block sizes with an odd step (18, 3, 6) are refused by the script: a
  line cannot balance on an odd side.

## Not built: every square at every offset

If a 6x6 at every shift must be a puzzle, every row has to repeat every 6
cells, and then a 12x12 or 8x8 square holds two identical rows, which the
rules forbid. This is a derivation, not a measurement.
