# Changelog

All notable changes to binsolve are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions
follow [semantic versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] — unreleased

First release. binsolve replaces the archived C# `BinaryPuzzleSolver`,
which could not finish puzzles its six strategies did not crack — its
backtracking was dead code that returned `false` immediately.

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
- `binsolve-tui` replays a solve step by step at an adjustable speed,
  with the current cell highlighted and honest timing statistics.

### Quality

- 89 tests: unit, property, frozen format vectors, a corpus of 20 real
  published puzzles, boundary deductions per composite type, determinism,
  performance thresholds, end-to-end CLI runs against the real binary,
  and a trace-replay property.
- Fuzzed: 1.59 billion parser executions and 7.5 million solver
  executions, both clean on the shipped code. Two real defects were
  found by fuzzing and fixed test-first.
- Performance, measured in release: worst puzzle 1.2 ms, median 98 µs,
  1,000-puzzle batch 188 ms — against targets of 1 s, 50 ms and 30 s.
- CI on Ubuntu, Windows and Arch, with a separate job asserting the
  performance thresholds and another asserting the solver core has no
  runtime dependencies.

### Known limitations

- Windows is build-verified, not runtime-verified: see
  `docs/WINDOWS_TEST_CHECKLIST.md`.
- The atomic-write retry for a file held open by another process has
  never executed anywhere; it is checklist-covered only.
- The solver fuzz target skips grids larger than 10×10, so composite
  types are not fuzzed.
