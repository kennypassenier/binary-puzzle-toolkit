# Test Plan — binsolve

What is proven, where, and what is deliberately not. Written in Phase 7
after a `test-gap-auditor` pass; the open questions become accepted
limitations or closed gaps once the Phase 7 gate is answered.

## Suites

| Suite | Location | Proves |
|---|---|---|
| Unit — grid | `bpt-core/src/grid.rs` | cell/char round-trip, row-major indexing, line serialization (K7, AR2) |
| Unit — regions | `bpt-core/src/region.rs` | the six region decompositions, solution and givens validation, a named violation per rule family (K2, K6, M3) |
| Unit — parser | `bpt-core/src/parse.rs` | tag and size inference, CRLF tolerance, multi-byte rejection, a remedy in every malformed-input message (K7) |
| Unit — strategies | `bpt-core/src/strategy.rs` | every strategy against the scenarios inherited from the C# README, plus the completion-feasibility check (K3) |
| Unit — search | `bpt-core/src/search.rs` | the fixpoint ladder, contradiction on conflicting deductions, trace stability (K3, K6, K16) |
| Unit — events | `bpt-core/src/event.rs` | trace step formatting (K16) |
| Format vectors | `bpt-core/tests/format_vectors.rs` | the frozen grammar as files: 8 valid lines round-trip, 7 invalid lines fail with the exact expected class (AR7, K7) |
| Round-trip property | `bpt-core/tests/roundtrip.rs` | parse→serialize identity over random grids of all six kinds; arbitrary strings never panic (K7) |
| Corpus meta-test | `bpt-core/tests/corpus.rs` | every corpus file parses; every published solution validates against its full region decomposition (M4, AR3) |
| Solve | `bpt-core/tests/solve.rs` | the C# cascade scenario and its refuted branch, the uniqueness search on a composite, multiple-solution reporting, every contradiction reason, corpus inventory (K4, K5, K6, M4) |
| Specials | `bpt-core/tests/specials.rs` | boundary deductions per type: across a block seam (whole-grid only), a block quota (block only), inner and outer nested regions (K2a–K2e) |
| Strategy-only sweep | `bpt-core/tests/sweep.rs` | every corpus puzzle solved by strategies alone matches its published solution; no false contradictions (M1) |
| Difficulty | `bpt-core/tests/difficulty.rs` | grading against the site's own labels: 19/20 exact, never more than one band off (M2) |
| Determinism | `bpt-core/tests/determinism.rs` | identical traces across runs for ladder and search puzzles and the whole corpus; one full trace pinned as a fixture (AR13, K16) |
| Thresholds | `bpt-core/tests/thresholds.rs` | the G5 targets in release, plus three adversarial sparse inputs with per-input budgets (K13) |
| Benchmarks | `bpt-core/benches/solve.rs` | trend data per puzzle, per type, uniqueness mode, 1,000-puzzle batch (K13) |
| Fuzzing | `bpt-core/fuzz/` | the parser never panics and round-trips what it accepts; the solver terminates and never reports an invalid solution (M7) |
| CLI E2E | `bpt/tests/cli.rs` | the real binary on real files: single and batch, 1:1 line mapping including blank lines, markers, exit codes, CRLF, atomic `--out`, both `--explain` channels, `--check`, `--no-backtrack`, `--unique` (K8–K12, K16, M1, M3) |
| Atomic write | `bpt/src/output.rs` | replacing an existing file leaves no temp behind (AR11, happy path) |
| TUI render model | `bpt-tui/src/replay.rs`, `ui.rs` | stepping, reversibility, backtracks restoring the pre-guess frame, stats accumulation, grid/step/stats rendering (K15, AR9) |
| TUI frames | `bpt-tui/tests/render.rs` | full frames through ratatui's in-memory backend, including the largest grid and an unsolvable puzzle (K15) |
| Trace replay property | `bpt-tui/tests/replay_property.rs` | replaying a recorded log reproduces the solver's own solution, over the corpus and over grids that require guessing (K16, AR9) |

CI runs formatting, clippy with warnings-as-errors and the full suite on
Ubuntu, Windows and an Arch container. A separate job asserts the
release-mode performance thresholds on Linux and Windows; another
asserts the solver core has no runtime dependencies.

## Two defects these suites did not catch, and now do

Both were found by the Phase 7 audit, and both got a failing test before
their fix:

- **Blank lines broke the 1:1 line mapping.** They were filtered out of
  the input, so every later output line described a different input line
  than its position claimed. Pinned by
  `k9_blank_lines_keep_the_line_mapping`.
- **The replay showed cells the solver never held.** Undoing a guess
  cleared only the guessed cell, leaving the refuted branch's deductions
  on screen — the displayed grid could even break the rules. Pinned by
  `k15_backtracking_restores_the_frame_before_the_guess` and the new
  trace-replay property test.

## Not covered, by decision

*Decided by Kenny at the Phase 7 gate, 2026-08-28.*

- **Windows runtime behaviour.** Build-verified only: CI compiles and
  runs the suite on `windows-latest`, but nobody has driven the binaries
  on a real Windows desktop. Specifically unproven: the atomic-write
  sharing-violation retry (the only reason that code exists), console
  encoding of the `—` and `·` characters, the TUI under crossterm, and
  paths with spaces or UNC prefixes. Mitigation:
  `docs/WINDOWS_TEST_CHECKLIST.md`, unsigned.
- **The fuzzers' size bound.** The solver fuzz target skips grids larger
  than 10×10, so no composite type is ever fuzzed (they are 12–18 wide);
  the parser target skips inputs over 4 KiB. Parsing is linear, so the
  second is low risk; the first is a real, bounded blind spot.
- **The atomic-write failure paths.** The retry loop has never executed
  on any platform, and it reacts to any I/O error rather than only to a
  sharing violation. Disk-full and killed-mid-write are not simulated.
- **The TUI event loop.** Key handling, raw-mode entry and exit, and
  terminal restoration after a panic have no tests; the render model
  they drive is covered instead.
- **The difficulty grading's guess band.** All 20 corpus puzzles need
  zero guesses, so the rule "any guessing means very hard" is calibrated
  against no data, and the Easy band rests on seven files whose names
  carry no site puzzle ID. Deferred deliberately: the grade is a sorting
  aid, not a correctness claim. Revisit when the corpus gains puzzles
  that force guessing.

## Closed at the Phase 7 gate

- **AR11's orphan `.tmp` cleanup** is now implemented and pinned by
  `ar11_a_successful_write_sweeps_an_orphaned_temp`: a successful write
  sweeps a leftover temp file for its own destination.
- **The terminal display (K11)** is now a pure function taking the
  terminal decision as a parameter, so all four outcome shapes are
  tested — including the guarantee that a pipe receives nothing but
  canonical lines.
- **Two unused Phase-3 choices** were dropped: `thiserror` was a
  declared dependency of the CLI with zero references, and `insta` was
  never a dependency at all. The pinned trace fixture covers what
  snapshot testing was chosen for.

## Still open

- M3's "verify a filled grid" is not implemented — `--check` requires a
  puzzle+solution file. Implement, or amend the feature text.
- K1 promised at least two real puzzles per standard size; 14×14 has one,
  and it is the easy one.
