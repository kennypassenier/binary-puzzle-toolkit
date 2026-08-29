# Changelog

All notable changes to BinaryPuzzleToolkit are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions
follow [semantic versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] — 2026-08-30

First release. One toolkit that solves binary puzzles and generates
them, merged on 2026-08-28 from two projects designed for each other:
the solver (binsolve) and the generator (binforge).

It replaces the archived C# `BinaryPuzzleSolver`, which could not
finish puzzles its six strategies did not crack — its backtracking was
dead code that returned `false` immediately.

### Solving

- Solves any binary puzzle (Takuzu/Binairo) with an even side length,
  and all five composite types from binarypuzzle.com: `4x6x6`, `4x8x8`,
  `9x6x6`, `8in14` and `6in10in14`. Composite types are modelled as one
  grid with overlapping constraint regions, so every sub-grid and the
  whole grid are satisfied at once.
- Six named strategies in cost tiers, each deduction attributable to a
  rule and a cell. Where the strategies stall, a depth-first search with
  cheap-tier propagation finishes the job, so every solvable puzzle is
  solved.
- Proves a solution unique by continuing the search past the first one.
  This makes a scraped puzzle self-validating: exactly one solution
  means the answer is verified without a published one.
- Reports an unsolvable puzzle with the rule it broke and where, never
  as a generic failure.
- Grades difficulty from the reasoning actually required, in three
  bands calibrated against the site's own labels (19 of 20 exact). Easy
  and medium are deliberately merged: no measurement here separates
  them.

### Interface

- One puzzle per line, rows written end to end, `.` for empty, with an
  optional type tag. Output mirrors the input with the dots filled in,
  so output line N always describes input line N — blank and malformed
  lines keep their slot behind a marker rather than shifting the rest.
- `--file`, `--out` (written atomically), `--unique`, `--no-backtrack`,
  `--check`, and `--explain` to stderr or `--explain=FILE`.
- Deterministic exit codes: `0` all solved, `1` at least one failed, `2`
  usage or file error.
- On a terminal a single puzzle also shows the grid and statistics;
  pipes and files always receive the canonical line only.
- `bpt-tui` replays a solve step by step at an adjustable speed, with
  the current cell highlighted and honest timing statistics.

### Generating

- Generates puzzles for any supported size or type, each **proven to
  have exactly one solution** before it is emitted — which is what makes
  a generated puzzle worth having without an answer key.
- Four difficulty levels measured from the reasoning a puzzle actually
  needs, not estimated. `--level` behaves as a target in practice:
  measured over 40 seeds on a 10x10, a ceiling of L1 produced 40 L1
  puzzles and L2 produced 40 L2.
- `--clues N` stops carving at a clue count; `--symmetry rotational` or
  `mirror` lays the clues out symmetrically.
- The same seed and version always produce the same puzzles, and a batch
  runs on all cores with byte-identical output to a single-threaded run.
- Batches (`--out-dir`) commit completely or not at all: a two-line file
  per puzzle, a flat file the solver validates in one run, and a
  manifest recording the seed, index and attempt that rebuild each
  puzzle. `--force` adds to an existing corpus and refuses anything
  already in it.
- Ctrl-C finishes the puzzle in flight, writes what is done with
  `status: cancelled`, and exits 3.

### Invented puzzle types

- A type is data, not code: a grid size plus rectangular regions in a
  file. Every rule walks the region list, so a type nobody has written
  yet needs no new code.
- A type name describes its own layout — `<n>x<a>x<a>` is n blocks of
  a×a, `<term>in<size>` centres that term in a larger square, and the
  two compose. So `4x6x6in16` works the moment it is written, at both
  ends of the format. The same grammar reproduces all five published
  types exactly.
- Layouts whose placement is a choice rather than a consequence of a
  name — overlapping regions, regions side by side — are supplied as a
  geometry file to both halves instead.
- `bpt inspect` draws any layout and says whether it is structurally
  valid, including refusing a region too flat to keep its lines
  distinct.

### Bounded work

- Every uniqueness question inside a carve is bounded by a node budget:
  search steps, never wall-clock, so the same seed gives the same puzzle
  on any machine and on any number of cores. Before it, two of five
  measured 18x18 seeds never finished, one still running after eighty
  minutes; after it, all five finish. Where the budget is reached the
  puzzle keeps a clue it might not have needed, and every manifest
  records that per puzzle.

### Quality

- 201 tests: unit, property, frozen format vectors, a corpus of 20 real
  published puzzles, boundary deductions per composite type, determinism,
  performance thresholds, end-to-end CLI runs against the real binary,
  and a trace-replay property.
- Fuzzed: 1.59 billion parser executions and 7.5 million solver
  executions, both clean on the shipped code. Two real defects were
  found by fuzzing and fixed test-first.
- Performance, measured in release: worst puzzle 1.2 ms, median 98 µs,
  1,000-puzzle batch 188 ms — against targets of 1 s, 50 ms and 30 s.
- Independent validation: generated batches are proven unique by the
  solver — a different code path from the generator — through the real
  binary. In CI that covers every geometry; a sabotage test confirms the
  harness fails on an ambiguous puzzle.
- A restore drill regenerates a committed batch from its manifest alone
  and compares byte for byte, run from a fresh `git clone` in CI.
- Ten CI jobs on Ubuntu, Windows and Arch: the gates, performance
  thresholds, the generation baseline, the validation sweep, the restore
  drill, coverage as information, and two purity checks — that the core
  has no runtime dependencies at all, and that neither library contains
  file access, threads, printing or a clock read.

### Known limitations

- Windows is build-verified, not runtime-verified: see
  `docs/solve/WINDOWS_TEST_CHECKLIST.md`. By the project's own rule that
  is beta until the checklist is signed.
- Generating the largest grids is slow and varies enormously between
  seeds: a 16x16 has been measured at both 3 s and 15 minutes. It always
  finishes, but "finishes" is the promise, not "quickly".
- Two runs writing the same output directory at once have no lock; see
  `docs/TEST_PLAN.md` for this and every other deliberate gap.
- The atomic-write retry for a file held open by another process has
  never executed anywhere; it is checklist-covered only.
- The solver fuzz target skips grids larger than 10×10, so composite
  types are not fuzzed.
