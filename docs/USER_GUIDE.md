# User guide

`bpt` does two things: it **solves** binary puzzles (Takuzu, Binairo)
and it **generates** them. Everything below was run against the binary
while writing this; the outputs are real.

## The rules, in one paragraph

A binary puzzle is a square grid of 0s and 1s. Every row and every
column holds as many 0s as 1s, never three of the same digit in a row,
and no two rows are identical (nor two columns). The composite types add
regions — blocks or nested squares that must satisfy the same rules on
their own — which is what makes them harder than their size suggests.

## Solving

```
bpt solve "1..0.0..1...0..1.1....0....1.0..11.."
```

One puzzle per line, a dot for an empty cell, rows written one after
another. The answer comes back in the same shape.

Useful flags:

| Flag | What it does |
|---|---|
| `--unique` | prove there is exactly one solution instead of stopping at the first |
| `--no-backtrack` | use human strategies only, never guess — tells you whether a puzzle is solvable by reasoning alone |
| `--explain` | print the reasoning to stderr; `--explain=FILE` to a file |
| `--file FILE` | one puzzle per line; output line N describes input line N |
| `--out FILE` | write results to a file, atomically |
| `--check` | verify puzzle+solution files instead of solving them |
| `--geometry FILE` | supply the regions for a type the line format cannot name |

A special type is marked with a prefix: `4x8x8:110...`. Plain grids need
no prefix — the size follows from the length.

Exit codes: 0 all solved · 1 one or more failed · 2 usage or file error.

## Generating

```
bpt forge --kind 10 --count 5
```

Every generated puzzle is **proven to have exactly one solution** before
it is emitted. That proof is what makes a generated puzzle worth having:
you can hand it to someone without an answer key.

| Flag | What it does |
|---|---|
| `--kind` | a size (`10`), a published type (`4x8x8`), or a composed name (`4x6x6in16`) |
| `--geometry FILE` | a layout no name can describe — overlapping or side-by-side regions |
| `--count` | how many |
| `--seed` | the same seed always produces the same puzzles |
| `--level` | L1 to L4, the hardest reasoning a solver should need |
| `--clues N` | stop carving once N clues remain |
| `--symmetry` | `rotational` (half turn) or `mirror` (left to right) |
| `--out FILE` | one line per puzzle |
| `--with-solutions` | add each solution on its own `solution:` line |
| `--out-dir DIR` | write a whole batch: see below |

### The four levels

The level is the hardest reasoning a solver needs, measured on the
puzzle itself rather than guessed at:

- **L1** — local patterns and counting per line suffice.
- **L2** — cross-line reasoning needed.
- **L3** — enumerating the possibilities for a line needed.
- **L4** — no amount of forced reasoning finishes it; guessing required.

`--level` is a ceiling, and in practice it lands on the level asked for:
measured over 40 seeds on a 10x10, a ceiling of L1 produced 40 L1
puzzles, L2 produced 40 L2, L3 produced 16 L3 and 24 L2, L4 produced 31
L4.

### Composed type names

A name can describe its own layout, so a type nobody has ever generated
before works the moment you write it:

- `<n>x<a>x<a>` — n blocks of a×a laid out √n by √n. `4x6x6` is four 6x6
  blocks in a 12x12; `9x6x6` is nine of them in an 18x18.
- `<term>in<size>` — that term centred in a larger square, chainable.
  `8in14`, `6in10in14`, and composed: `4x6x6in16` is a four-quadrant
  12x12 centred in a 16x16.

A name outside the grammar is refused rather than guessed at, because
reading `9x6x6` as a plain 18x18 would answer a different puzzle.

Layouts whose *placement* is a choice — two regions side by side, two
that overlap — cannot be named and travel as a geometry file instead:

```
bpt forge --geometry geometries/overlap8in12.toml --count 3 --out p.txt
bpt solve --file p.txt --geometry geometries/overlap8in12.toml --unique
```

### Batches

```
bpt forge --kind 10 --count 100 --seed 42 --out-dir corpus/generated
```

That writes, all at once or not at all:

- one two-line file per puzzle, named `bf-<seed>-<index>-<level>.txt`,
  holding the puzzle and its solution
- `puzzles.txt`, one puzzle per line, which is what
  `bpt solve --file … --unique` validates in a single run
- `manifest.json`, recording for every puzzle the three numbers that
  regenerate it — seed, index, attempt — plus its level, clue count and
  a fingerprint

A directory belongs to its batch: a second run refuses unless you pass
`--force`, which then *adds* to it, refusing any puzzle already there.
All the puzzles stay in the manifest and in `puzzles.txt`; one directory
holds one geometry at one level.

Ctrl-C finishes the puzzle in flight, writes what is done with
`status: cancelled` and exits 3. A second Ctrl-C aborts.

Exit codes: 0 everything requested produced · 1 fewer than requested ·
2 usage or file error · 3 cancelled.

## Inspecting a geometry

```
bpt inspect 4x6x6
bpt inspect geometries/overlap8in12.toml
bpt inspect 4x6x6in16
```

Draws the grid with a letter per region, lists the regions, and says
whether the layout is structurally valid — which catches a mistyped
origin far faster than wondering why nothing generates.

## Watching

```
bpt watch "1..0.0..1...0..1.1....0....1.0..11.." --speed 20
```

Replays the solve step by step in the terminal.

## When generation is slow

Generating grows steeply with size. Measured on a fast machine, one
puzzle at L4: 5 ms at 10x10, 146 ms at 14x14, seconds at 16x16, and up
to a minute or two at 20x20. Batches use all cores and produce exactly
the same puzzles as a single-threaded run.

Every uniqueness question inside the carve is bounded, so generation
always finishes. Where that bound is reached the puzzle keeps a clue it
might not have needed — still uniquely solvable, just not minimal — and
the manifest records it per puzzle as `budget_hits`.
