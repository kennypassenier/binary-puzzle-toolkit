# Scope — binforge

Phase 0 outcome, approved by Kenny on 2026-08-12 (gate form, all items
approved unchanged; name decided in the same form). Successor to
`~/Projects/BinaryPuzzleScraper` (C#, archived): the scraper is retired
and replaced by a **generator** — this is a reconception, not a port.
Companion project to `~/Projects/binsolve` (the solver); the two share
the one-line dot-format contract (binsolve G6).

## Goals

- **G1 · Standard puzzles, any even size.** Generate standard binary
  puzzles (Takuzu/Binairo: no three identical adjacent in a row or
  column; equal 0/1 count per row and column; all rows unique; all
  columns unique) for any even side length. Covers binarypuzzle.com's
  6x6–14x14 and larger (16x16, 20x20, …); the practical ceiling is set
  by the G8 performance targets, never hardcoded. *(S1)*
- **G2 · All five known special types.** `4x6x6`, `4x8x8`, `9x6x6`,
  `8in14`, `6in10in14` — the same geometry definitions binsolve G2
  models as one grid with overlapping constraint regions, so binsolve
  accepts the output as-is. *(S2)*
- **G3 · Data-driven geometry model.** A puzzle type = grid size + a
  list of rectangular regions that must each satisfy all Takuzu rules.
  Standard and all five specials are instances. New/invented types
  (e.g. `4x10x10`, `8in12in16`, overlapping-band compositions) are
  config entries, not Rust changes. Infeasible geometries are detected
  (solver cannot complete an empty grid) and rejected with a clear
  message. *(S3)*
- **G4 · Uniqueness guarantee.** Generation = fill an empty grid to a
  full valid solution (randomized solver), then carve: remove clues one
  at a time, re-proving after each removal that exactly one solution
  remains; a breaking removal is reverted. Every emitted puzzle is
  proven to have exactly one solution. Uses binsolve G4's uniqueness
  prover. *(S4)*
- **G5 · Own four-tier difficulty scale.** Difficulty = the lowest
  technique tier that cracks the puzzle, measured from the solve trace:
  tier 1 direct fills; tier 2 row/column counting interplay; tier 3
  unique-row/column eliminations and short contradiction probes;
  tier 4 deep contradiction chains. Own consistent scale; matching
  binarypuzzle.com's labels is explicitly not a goal. *(S5)*
- **G6 · Difficulty targeting.** Request size/type + tier and the
  carving loop keeps adjusting (clue selection, fresh-solution retries)
  until the measured tier equals the requested tier; output always
  reports the measured tier. *(S6)*
- **G7 · Output contract: binsolve dot-format.** One puzzle = one line,
  `.` for empty, prefix tag for specials (`4x8x8:110..0.1…`), exactly
  binsolve G6; solutions emitted alongside in the same format.
  Generated puzzles flow straight into binsolve's corpus and CLI.
  binsolve's tag vocabulary is frozen at the five known types: before
  any invented type is emitted, its tag is added via a formal
  mini-round on binsolve's frozen S6b decision. *(S7)*
- **G8 · Performance.** Repeatable benchmark on Kenny's PC: one
  very-hard 14x14 in < 10 s; a 100-puzzle 10x10 batch in < 60 s; any
  single special type in < 30 s. Larger invented sizes may take longer;
  their measured cost is documented, not capped. *(D3)*

## Non-goals

- **N1 · No scraping.** BinaryPuzzleScraper is retired without
  replacement; binarypuzzle.com is not a data source (no fetching, no
  selectors, no daily sync). Accepted consequence: no externally
  labeled corpus for difficulty calibration (G5 stands on its own).
  *(S/N1)*
- **N2 · No backend, website, API or remote logging.** The old
  WebsiteService (passenier.be endpoints, log posting) disappears
  entirely; output is files on disk. *(S/N2)*
- **N3 · No play interface.** Interactive play belongs to binsolve's
  stretch TUI (G7) or a future consumer of the generated files.
  *(S/N3)*
- **N4 · No publishing or packaging.** Personal tool, single user; no
  crates.io, no installers. Cross-platform (C3) is in scope,
  distribution work is not. *(S/N4)*

## Constraints

- **C1 · Rust (stable), library core + thin CLI.** Generator library
  crate with zero ambient I/O; thin CLI binary on top. Specific crates
  are a Phase 3 decision. *(S/C1)*
- **C2 · Fully offline.** No network code at all; no credentials, no
  endpoints. *(S/C2)*
- **C3 · Linux + Windows.** Both platforms build, run and are covered
  by CI — mirrors binsolve so the two tools work side by side.
  *(S/C3)*
- **C4 · binsolve-core is the solving engine.** Path dependency for
  solving, uniqueness proving and the strategy trace feeding G5; zero
  solver logic duplicated here. Known risk, accepted at the gate:
  binsolve is mid-build (Phase 6); generator milestones that need the
  solver wait for, or contribute to, binsolve's progress — its trace
  and uniqueness features become hard requirements on binsolve's plan.
  *(S8)*

## Success criteria ("done")

1. **Validated batches for every type:** 100 puzzles per standard size
   (6–20) and per special type pass independent validation — the
   binsolve CLI (not our own code path) solves each one and confirms
   exactly one solution. Generator/validator disagreement = bug.
   *(D1)*
2. **Difficulty targeting always lands:** every emitted puzzle's
   measured tier equals the requested tier (bounded by the G8 time
   budget); "couldn't reach tier 4 for this geometry" is a reported
   outcome, never a silently wrong label. *(D2)*
3. **G8 performance targets hold** in a repeatable benchmark. *(D3)*
4. **At least two invented types end-to-end:** two types the site does
   not have (candidates `4x10x10`, `9x8x8`, `8in12in16`; final pick in
   Phase 2) generated, tagged and solved end-to-end by binsolve after
   the G7 tag mini-round. *(D4)*

## Decision log

| Date | Decision |
|---|---|
| 2026-08-12 | Scope approved (all items unchanged); direction = generator only, scraping dropped |
| 2026-08-12 | Project name: **binforge** |
| 2026-08-12 | Location: fresh repo at `~/Projects/binforge`, old scraper repo stays archived |
