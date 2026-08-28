# User guide

Every command and every piece of output on this page was run against the
release binary before it was written down. Build it first:

```
cargo build --release
```

The examples use `binsolve` for `./target/release/binsolve`.

## The puzzle format

One puzzle is one line: the rows written out end to end, `0` and `1` for
filled cells, `.` for empty. A 6×6 is 36 characters, an 8×8 is 64, and
so on — the size is inferred from the length.

```
1..0....00.1.00..1......00.1...1..00
```

The five composite types from binarypuzzle.com cannot be told apart from
their length alone, so they carry a tag and a colon:

| Tag | Type | Grid |
|---|---|---|
| `4x6x6` | four 6×6 blocks | 12×12 |
| `4x8x8` | four 8×8 blocks | 16×16 |
| `9x6x6` | nine 6×6 blocks | 18×18 |
| `8in14` | an 8×8 centred in a 14×14 | 14×14 |
| `6in10in14` | a 6×6 in a 10×10 in a 14×14 | 14×14 |

```
4x8x8:.10...01.......0.....0.1....0...0..0...
```

For these, every sub-grid *and* the whole grid must satisfy all the
rules at once.

## Solving one puzzle (K8)

*↳ K8 = solving a single puzzle passed straight on the command line.*

```
$ binsolve "1..0....00.1.00..1......00.1...1..00"
101010010011100101011010001101110100
```

In a terminal you also get the grid and what it took:

```
1 0 1 0 1 0
0 1 0 0 1 1
1 0 0 1 0 1
0 1 1 0 1 0
0 0 1 1 0 1
1 1 0 1 0 0
solved in 8.5µs — 22 deductions, 0 guesses, 0 backtracks, graded easy
101010010011100101011010001101110100
```

That extra display appears **only** when the output goes to a terminal.
Pipe or redirect it and you get the single canonical line, so scripts
never have to strip anything.

A composite type keeps its tag in the answer:

```
$ binsolve "8in14:...1.1...0.1...1..0..."
8in14:001101010011011101001011001001001101101010001100110101…
```

## Solving a batch (K9, K10)

*↳ K9 = reading a file of puzzles, one per line, where output line N
always describes input line N. ↳ K10 = writing the results to a file
instead of the screen.*

```
$ binsolve --file puzzles.txt
$ binsolve --file puzzles.txt --out solutions.txt
```

The 1:1 mapping is the point: line 47 of the output is always the answer
to line 47 of the input, so you can match results back to a source list
by position. Lines that cannot be solved keep their slot and their
original text behind a marker:

```
$ binsolve --file mixed.txt
101010010011100101011010001101110100
#contradiction:000.................................
#invalid:rommel
```

Blank lines also keep their slot, reported as `#invalid:` — skipping
them would shift every later line onto the wrong puzzle.

Files are written atomically: an interrupted write leaves the previous
file intact rather than half a result.

## Understanding the output

| Line starts with | Meaning | What to do |
|---|---|---|
| `0`/`1` (or a tag) | solved | nothing — that is the answer |
| `#invalid:` | the line is not a valid puzzle | run it with `--explain` to see which character or length is wrong |
| `#contradiction:` | the puzzle has no solution | a given was probably misread; compare with the source |
| `#multiple:` | more than one solution exists | a published puzzle never does — a given was probably lost |
| `#stuck:` | only with `--no-backtrack`: strategies ran out | drop the flag and the search will finish it |

Exit codes: `0` everything solved, `1` at least one puzzle failed, `2`
the command or the file could not be used.

## Proving a solution unique (K5)

*↳ K5 = continuing the search past the first solution to prove no other
exists.*

This is what makes a scraped puzzle self-validating: if exactly one
solution exists, that solution *is* the verified answer, no published
answer needed.

```
$ binsolve --unique "1..0....00.1.00..1......00.1...1..00"
101010010011100101011010001101110100

$ binsolve --unique "................"
#multiple:................
```

## Seeing the reasoning (K16)

*↳ K16 = a numbered, human-readable list of the steps the solver took.*

The bare flag writes the steps to stderr, so stdout stays canonical:

```
$ binsolve --explain "1..0....00.1.00..1......00.1...1..00" 2> trace.txt
101010010011100101011010001101110100

$ head -3 trace.txt
step 1: FindDuo — r1c1 = 1 (cells r1c2 and r1c3 are both 0, neighbours must differ)
step 2: FindDuo — r1c4 = 1 (cells r1c2 and r1c3 are both 0, neighbours must differ)
step 3: FillByCount — r1c0 = 0 (row 1 already has its 3 1s, the rest get the opposite)
```

To write them straight to a file, the equals sign is required — without
it the puzzle argument would be read as the filename:

```
$ binsolve --explain=trace.txt "1..0....00.1.00..1......00.1...1..00"
```

Guesses and backtracks appear as their own steps, so a trace doubles as
a story of how hard the puzzle was.

## Solving without guessing (M1)

*↳ M1 = using only the human-style strategies, never falling back on
trial and error.*

```
$ binsolve --no-backtrack "................"
#stuck:................
```

Useful for finding out whether a puzzle is solvable by reasoning alone,
and for spotting which strategies are missing. In a terminal it also
shows the partial grid and how far it got.

## Checking a puzzle against its solution (M3)

*↳ M3 = verifying an existing answer rather than computing one.*

`--check` reads a two-line file: the puzzle, then `solution:` followed by
the full grid.

```
$ binsolve --check --file corpus/standard/6/bp-s6l1n1-20260812-easy.txt
ok: corpus/standard/6/bp-s6l1n1-20260812-easy.txt
```

A wrong solution prints one `invalid:` line per violated rule, naming the
rule and where it broke, and exits `1`.

## Difficulty grading (M2)

*↳ M2 = an estimate of how hard a puzzle is, from the reasoning it
actually required.*

The grade appears in the terminal statistics line:

```
solved in 8.5µs — 22 deductions, 0 guesses, 0 backtracks, graded easy
solved in 36.7µs — 74 deductions, 0 guesses, 0 backtracks, graded hard
```

Three bands, not four:

| Grade | Means | Matches the site's |
|---|---|---|
| easy | local patterns and line counts sufficed | easy *and* medium |
| hard | cross-line reasoning was needed | hard |
| very hard | line enumeration or guessing was needed | very hard |

Easy and medium are deliberately merged: measured across the corpus,
both fall to the same strategies, so nothing here can separate them.
Claiming four bands would be inventing a distinction the solver cannot
make.

## Watching a puzzle being solved (K15)

*↳ K15 = the terminal viewer that replays a solve step by step.*

```
$ binsolve-tui "1..0....00.1.00..1......00.1...1..00"
$ binsolve-tui --file puzzles.txt --speed 40
```

The puzzle is solved at full speed first and then replayed, so the timing
statistics stay honest. Givens are bold, deduced cells cyan, and the cell
of the current step is highlighted.

| Key | Does |
|---|---|
| space | play / pause |
| ← → | one step back / forward |
| ↑ ↓ | previous / next puzzle |
| + - | double / halve the replay speed |
| Home End | jump to the start / the end |
| q or Esc | quit |

## Limits worth knowing

- **Windows is build-verified, not runtime-verified.** CI compiles and
  tests binsolve on Windows every push, but nobody has driven the
  binaries on a real Windows desktop yet. See
  [WINDOWS_TEST_CHECKLIST.md](WINDOWS_TEST_CHECKLIST.md).
- **binsolve never touches the network.** Puzzles come from arguments or
  files, results go to the screen or a file. There are no credentials and
  no telemetry.
- **It solves, it does not generate.** Creating new puzzles is the job of
  its companion project.

## Where to go next

- [OPERATIONS_RUNBOOK.md](OPERATIONS_RUNBOOK.md) — numbered procedures
  for batches, verification and maintenance.
- [DEBUGGING_GUIDE.md](DEBUGGING_GUIDE.md) — symptom → cause tables for
  every error message.
- [ARCHITECTURE_REFERENCE.md](ARCHITECTURE_REFERENCE.md) — how the solver
  actually works.
