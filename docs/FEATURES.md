# Features — binsolve

**FROZEN 2026-08-12** (Phase 2 gate: two rating rounds, two deep-dive
rounds, freeze report approved). Changes only via mini-rounds
(FORM_PROTOCOL §5); amendments are dated notes below the affected
feature. IDs are permanent and appear in commits and test names.

## Tally

| Rating | Count | IDs |
|---|---|---|
| Essential | 17 | K1 K2a-e K3 K4 K5 K6 K7 K8 K9 K10 K12 K13 K14 K15 M3 M4 M7 |
| Desired | 5 | K11 K16 M1 M2 M5 |
| Later | 1 | M6 |
| Don't do | 0 | — |

## Essential

### K1 · Standard n×n solving engine
Solve any even-sided square grid (site sizes 6x6–14x14; engine is
size-generic). Rules: no three identical adjacent in row/column; equal
0/1 count per row and column; rows mutually unique; columns mutually
unique.
**Tests:** unit tests per rule; ≥2 real site puzzles per size 6–14
verified against published solutions (incl. very hard); property test
that produced solutions satisfy all rules.

### K2 · The five special types (K2a–K2e)
One overlapping-constraint-regions mechanism covering:
| Sub-ID | Tag | Type |
|---|---|---|
| K2a | `4x6x6` | four 6x6 quadrants forming a 12x12 |
| K2b | `4x8x8` | four 8x8 quadrants forming a 16x16 |
| K2c | `9x6x6` | nine 6x6 blocks (3x3) forming an 18x18 |
| K2d | `8in14` | 8x8 centered in a 14x14 |
| K2e | `6in10in14` | 6x6 in 10x10 in 14x14, centered, doubly nested |
Sub-grids and whole grid must be simultaneously valid.
**Tests:** ≥2 real archive specials per type verified against site
solutions; region-boundary unit tests (deduction using only the inner
grid; deduction using only the outer grid).

### K3 · Logical strategy engine
Named human-style strategies applied in complexity order; the C#
project's six (FindDuo, AvoidTriple, SupplementLine, KeepLineUnique,
VirtualLimitReached, FillPossibilities) generalized and extended. Every
deduction is attributable (cell, strategy, reason) — feeds K16 trace,
M1/M2 and the K15 TUI.
**Tests:** focused unit tests per strategy; every scenario from the old
README as a regression case.

### K4 · Backtracking fallback (completeness)
Systematic search with propagation when strategies run dry; every
solvable puzzle solves (this was dead code in the C# version).
**Tests:** old README's cascade scenario solves; adversarial near-empty
grids solve; strategy-only vs full mode compared on the corpus.

### K5 · Uniqueness proof mode
Search continues past the first solution to prove it is the only one.
Self-validation for scraped puzzles: exactly one solution ⇒ verified
answer; more ⇒ bad-scrape flag.
**Tests:** known site puzzles prove unique; crafted two-solution grid
reports both; empty 4x4 reports "multiple" fast.

### K6 · Contradiction detection with reason
No-solution puzzles report a concrete reason, never a generic failure.
**Tests:** one crafted contradictory input per rule family, asserting
the specific reason text.

### K7 · Input parsing & validation
One-line dot format, optional prefix tag (`4x8x8:110..0…`); plain
puzzles untagged, size = √length. Malformed input errors name problem
AND remedy (standing rule 11).
**Tests:** parse→serialize round-trip property test; one test per
malformed-input class asserting the remedy message.

### K8 · Single puzzle via CLI argument
`binsolve "1..0.0..."`.
**Tests:** E2E against the real binary asserting stdout.

### K9 · Batch file input with 1:1 output mapping
One puzzle per line; output line N corresponds to input line N;
failures keep their tag with a status marker; batch never derails.
**Tests:** E2E with a real mixed file (solvable, contradictory,
malformed) asserting line-by-line correspondence.

### K10 · Output to console or file (canonical single-line)
Same format as input with dots filled; `--out FILE` writes atomically
(standing rule 12); pipes always get canonical lines.
**Tests:** E2E stdout-vs-file equivalence; piped output contains only
canonical lines.

### K12 · Machine-readable exit codes & status markers
Deterministic exit codes (all solved / some failed / bad input) and
in-file status markers; exact values frozen in Phase 4.
**Tests:** E2E asserting each exit code and marker.

### K13 · Performance benchmark harness
G5 targets (<1 s worst single puzzle, <50 ms typical, 1,000-puzzle
batch <30 s) over a fixed corpus.
**Tests:** benchmark suite in CI (informational trend) + release-mode
threshold test for the hard targets.

### K14 · Linux + Windows support
Both platforms build, run, and pass the full suite in CI on every push.
**Tests:** CI matrix (ubuntu-latest + windows-latest); E2E suite covers
path/newline handling on both.

### K15 · Ratatui TUI frontend
Live interactive view of one or more puzzles being solved, with
statistics (strategies used, time, backtrack count). Second binary over
the library core. *Upgraded from stretch goal to Essential at the
round-1 gate (2026-08-12) — "done" includes a working TUI.*
**Tests:** defined when designed in Phase 4/5 (at minimum: render-model
unit tests decoupled from the terminal).

### M3 · Verification mode (`--check`)
Verify a filled grid (or puzzle + candidate solution) against all rules
and the givens. Referee for scraped answers and the corpus.
**Tests:** valid solutions pass; each rule-violation class caught and
named.

### M4 · Curated real-puzzle test corpus
Committed fixtures: real binarypuzzle.com puzzles — every size, every
difficulty, all five special types, with published solutions where
available — in the K7 format. Data only; scraper code stays out (N3).
**Tests:** the corpus IS the test data; meta-test asserts every corpus
file parses.

### M7 · Parser & solver fuzzing
cargo-fuzz targets: parser (arbitrary bytes never panic) and solver
(arbitrary grids terminate). Runs on demand on Linux (nightly
toolchain, dev-only); not in per-push CI. Crashes become regression
tests (standing rule 8). *Rated Essential after deep-dive round
(2026-08-12).*
**Tests:** the fuzz targets + regression tests for any finding.

## Desired

### K11 · Pretty grid + stats on interactive terminal
Readable grid + short stats block when stdout is a TTY; scripts never
see it.
**Tests:** snapshot test of rendered grid; TTY-detection E2E.

### K16 · Solve trace (`--explain`)
Human-readable numbered steps (strategy, line/cell, reason), including
guesses and backtracks. Channel decision (K16a, 2026-08-12): stderr by
default, `--explain=FILE` writes to a file; never stdout (protects
K9's 1:1 mapping).
**Tests:** snapshot trace for a fixed puzzle; property test that
replaying a trace reproduces the solution.

### M1 · Strategy-only mode (`--no-backtrack`)
Solve with strategies alone; report solved or stuck-at-N%-filled. Tool
for discovering missing strategies; backbone of M2.
**Tests:** corpus comparison vs full mode; strategies-insufficient
puzzle reports "stuck", not failure.

### M2 · Difficulty grading
Per-puzzle difficulty estimate (highest strategy tier + guess/backtrack
count), calibrated against site labels on the corpus.
**Tests:** grading on labeled corpus puzzles correlates with site
labels (thresholds agreed in Phase 7).

### M5 · Parallel batch solving
Batch puzzles across cores, output order preserved. *Upgraded from my
"Later" recommendation to Desired at the round-2 gate.*
**Tests:** parallel output identical to sequential on the corpus.

## Phase 2 mandatory items (decided 2026-08-28)

These three became mandatory in the procedure after this feature list
was frozen, so they were run as a separate round.

### Update & distribution mechanism — rebuild from git
binsolve is updated by pulling the repository and running
`cargo build --release`. No self-update code, by decision: the tool is
developed on the same machines it runs on, so a self-updater would be
machinery without a recipient. The release workflow still publishes
built archives with checksums for a machine without a Rust toolchain.

### Backup & restore — the repository is the backup
State inventory: the source and the 20-puzzle test corpus live in git
and are pushed to GitHub; nothing else persists between runs (no
database, no settings, no secrets, no cache). Restore is `git clone`
plus `git config core.hooksPath .githooks`.
- **Automatic?** Yes — every commit is pushed; no separate mechanism.
- **Restore exercised?** Yes, 2026-08-28: the release binaries were
  installed into an empty directory and driven from there.
- The fuzz corpus (910 inputs) is deliberately NOT in git. Losing it
  costs a fuzzer some coverage-rediscovery time, not information.

### Ecosystem integration — open
Deferred to its own round: see `docs/PENDING_MINI_ROUNDS.md`.

## Later

### M6 · JSON output mode
`--json`: one JSON object per puzzle (grid, status, time, difficulty,
trace). No consumer today; revisit if the new website wants it.
