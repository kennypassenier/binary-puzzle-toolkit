# Scope — binsolve

Phase 0 outcome, approved by Kenny on 2026-08-12 (gate form + one
deep-dive round covering S6 details and the project name). Successor to
`~/Projects/BinaryPuzzleSolver` (C#, archived); this is a from-scratch
rewrite, not a port.

## Goals

- **G1 · Standard puzzles.** Solve any square binary puzzle
  (Takuzu/Binairo) with even side length. Covers binarypuzzle.com's
  6x6–14x14 but the engine is size-generic. Rules: no three identical
  digits adjacent in a row or column; equal count of 0s and 1s per row
  and per column; all rows mutually unique; all columns mutually
  unique. *(S1)*
- **G2 · All five special types.** Full sweep of binarypuzzle.com's
  special archive (Apr 2014 – Aug 2026, ~2,500 puzzles) shows exactly
  five types; all are in scope:
  | Tag | Type | Geometry |
  |---|---|---|
  | `4x6x6` | 4 times 6x6 | 12x12, four 6x6 quadrants |
  | `4x8x8` | 4 times 8x8 | 16x16, four 8x8 quadrants |
  | `9x6x6` | 9 times 6x6 | 18x18, nine 6x6 blocks (3x3) |
  | `8in14` | 8x8 in 14x14 | 8x8 centered in a 14x14 |
  | `6in10in14` | 6x6 in 10x10 in 14x14 | doubly nested, centered |
  Each sub-grid AND the whole grid must simultaneously satisfy all G1
  rules. The solver models one grid with multiple overlapping
  constraint regions — one mechanism covers all five types and any
  future member of the family. *(S2)*
- **G3 · Completeness.** Human-style logical strategies plus full
  constraint propagation and backtracking: every puzzle that has a
  solution gets solved. Contradictory puzzles are reported as such,
  with the reason — never a silent failure. (The old solver's
  backtracking was dead code; its known-unsolvable cascade scenarios
  must solve here.) *(S3)*
- **G4 · Uniqueness proof.** The solver can prove a solution is the
  only one (search continues past the first hit). A scraped puzzle
  without a published answer is thereby self-validating: exactly one
  solution found ⇒ that is the verified answer; two or more ⇒ bad
  scrape flag. *(S4)*
- **G5 · Performance.** Any single site puzzle (up to 18x18 very hard)
  in < 1 s, typical case < 50 ms; a 1,000-puzzle batch in < 30 s on
  Kenny's PC. Encoded as a repeatable benchmark, not a feeling. *(S5)*
- **G6 · I/O.** One puzzle = one line: rows chained, `.` for empty.
  Plain puzzles carry no tag (size = √length); special types carry a
  prefix tag from the G2 table, colon-separated:
  `4x8x8:110..0.1…`. Input: CLI argument or file (one puzzle per
  line). Output: same single-line format with dots filled in; line N
  of output corresponds to line N of input; failed puzzles keep their
  tag and get a status marker instead of a grid. Interactive terminal
  additionally gets a human-readable grid + stats; pipes/files always
  get the single-line form. Machine-readable behaviour (exit codes,
  markers) and exact syntax frozen in Phase 4 with regression vectors.
  The scraper project adopts this spec as its output contract.
  *(S6, deep-dive S6a/S6b)*
- **G7 · Stretch: Ratatui TUI.** Interactive terminal frontend showing
  live solving (single or multiple puzzles) with statistics
  (strategies used, time, backtrack count). Not required for "done";
  enabled by the library/CLI split in C1. *(S7)*

## Non-goals

- **N1 · No website or API integration.** The old WebsiteService
  (passenier.be fetch/PUT/remote logging) is dropped without
  replacement. A future site consumes file/CLI output. *(S8)*
- **N2 · No puzzle generator.** (If ever wanted: G4's uniqueness
  prover is the hard half of one — new scope round required.) *(S9)*
- **N3 · No scraper code in this repo.** The G6 format is the entire
  contract between the two projects. *(S10)*
- **N4 · No crates.io publishing or packaging.** Personal tool, single
  user. Amended at the gate: it must build and run on **both Linux and
  Windows** — cross-platform is in scope, distribution work is not.
  *(S11, adjusted)*

## Constraints

- **C1 · Rust (stable).** Solver core is a library crate with zero
  ambient I/O (no printing, no file access); CLI is a thin binary over
  it, the future TUI (G7) a second one. Specific crates are a Phase 3
  decision. *(S12)*
- **C2 · Fully offline.** No network code at all; no credentials, no
  endpoints. *(S13)*
- **C3 · Linux + Windows.** Both platforms build, run, and are covered
  by CI. *(S11 amendment)*

## Success criteria ("done")

1. Solves current and archive puzzles of every size, difficulty and
   all five special types from binarypuzzle.com, spot-verified against
   the site's published solutions.
2. Every solution passes the G4 uniqueness check.
3. G5 performance targets hold in a repeatable benchmark.
4. The hardest puzzle Kenny can throw at it does not stump it — a
   failing "very hard" is by definition a bug. *(S14)*

## Decision log

| Date | Decision |
|---|---|
| 2026-08-12 | Scope approved (S1–S14; S11 amended to include Windows) |
| 2026-08-12 | Output mirrors input format, single line, 1:1 line mapping (S6a) |
| 2026-08-12 | Prefix type tags with vocabulary `4x6x6 · 4x8x8 · 9x6x6 · 8in14 · 6in10in14`; plain puzzles untagged (S6b) |
| 2026-08-12 | Project name: **binsolve** (S15, round 2) |
