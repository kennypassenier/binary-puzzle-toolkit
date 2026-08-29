# Test plan

What is tested, where, and — just as important — what is deliberately
not. Written at the Phase 7 gate on 2026-08-29 and kept current with the
suites it describes.

This file covers the whole toolkit. `docs/legacy/TEST_PLAN.md` is the
solver half's own plan from before the merge and is kept for its
history; where the two disagree, this one is current.

## The suites

| Suite | Where | What it holds the code to |
|---|---|---|
| Core unit tests | `bpt-core/src/**` | grid, regions, rules, the strategy ladder, the search |
| Format vectors | `bpt-core/tests/format_vectors.rs` | the line format as files: eleven valid lines that must round-trip byte for byte, twelve invalid ones that must fail with a named error class |
| Special types | `bpt-core/tests/specials.rs` | each published type's rules firing where the whole grid is silent; the tag grammar reproducing every published type's regions |
| Corpus | `bpt-core/tests/corpus.rs` | twenty real puzzles from binarypuzzle.com, every size and all five special types, solved and proven unique |
| Determinism | `bpt-core/tests/determinism.rs` | the same input giving the same answer, bit for bit |
| Solver thresholds | `bpt-core/tests/thresholds.rs` | the G5 speed promises, release only |
| Generator unit tests | `bpt-forge/src/**` | filling, carving, grading, batches, the manifest, the node budget |
| Geometry | `bpt-forge/tests/geometry.rs` | every structural way a geometry file can be wrong, each with its remedy |
| Properties | `bpt-forge/tests/properties.rs` | over random geometries and seeds: every puzzle uniquely solvable, a puzzle is its solution minus cells, grading reproducible, every puzzle regenerable from its triple |
| Generator thresholds | `bpt-forge/tests/thresholds.rs` | the G8 speed promises, release only |
| Baseline | `bpt-forge/tests/baseline.rs` | measured medians and p95s per geometry, guarded against regression |
| CLI | `bpt/tests/cli.rs` | every subcommand end to end against the real binary |
| Batches | `bpt/tests/batch_cli.rs` | the batch layout, all-or-nothing, duplicates against an existing corpus, cancellation, symmetry, clue targets |
| Atomic writes | `bpt/tests/atomic.rs` | temp-then-rename, orphan cleanup, the Windows sharing-violation path |
| Validation | `bpt/tests/validation.rs` | generated puzzles proven unique through the real binary, plus a sabotage puzzle that must be caught |
| Restore drill | `bpt/tests/restore_drill.rs` | a committed batch rebuilt from its manifest alone, byte for byte |
| Invented types | `bpt/tests/invented_types.rs` | types defined only as data, generated and solved end to end |

CI additionally runs the D1 sweep (a batch of every geometry validated
through the binary), the restore drill from a fresh `git clone`, the
purity check (no I/O, threads or clock in either library), the
zero-dependency check on the core, and coverage as information.

## Not covered, by decision

These were found in the Phase 7 audit and consciously left open. Each is
recorded here verbatim rather than fixed, which is the point of the
list: a known hole is a decision, an unknown one is a surprise.

### Two runs writing the same directory at once (G2, accepted 2026-08-29)

Starting `bpt forge --out-dir` twice on the same directory has no lock.
The second run's orphan cleanup removes the temp files the first is
writing, which reaches the first as a failed rename; both would also
write their own manifest over the other's.

**Why accepted.** Kenny is the only user and the scenario needs him to
start the same command twice on purpose. The damage is a failed run —
loud — not a wrong result written quietly. A lock file would be small
work but it is machinery for a situation that does not arise.

### Windows is CI-green but never runtime-verified (G4, accepted 2026-08-29)

Windows compiles and the whole suite passes on a GitHub runner,
including the atomic-write path written specifically for it. Nobody has
ever started the toolkit on a real Windows machine.
`docs/solve/WINDOWS_TEST_CHECKLIST.md` is written and unsigned, so by
the procedure's own rule the honest state is **beta until that checklist
is signed**, not "done".

**Why accepted.** Kenny works on Garuda; Windows is carried because it
could be, not because it is used. Note that the Windows runner did earn
its place on 2026-08-29 by catching two real failures invisible locally
— a checkout that rewrote line endings, and a speed bound that a slower
machine broke.

### No security review was run (G5, accepted 2026-08-29)

The procedure makes `/security-review` mandatory for anything touching
secrets, network or authentication. This toolkit touches none of the
three: it is offline, has no authentication, and holds no secret. It
does read and write files, but every path comes from the command line —
no path is ever taken from a puzzle file, a geometry file or a manifest
and then written to.

**Why accepted.** The judgement above is recorded rather than the step
silently skipped, so it can be revisited if the toolkit ever grows a
network or a credential.

## Closed at this gate

- **G1 — a `--force` run made earlier puzzles invisible.** Measured:
  three puzzles, then two more with `--force`, left five files on disk
  but two lines in `puzzles.txt` and two entries in the manifest.
  Validation checked two of five; the restore drill restored two of
  five. A run now carries the existing batch forward, the seed moved
  onto each manifest entry so a directory can hold several runs, and a
  run that would add a different geometry or level is refused.

  The test that appeared to cover this did not: it read `puzzles.txt`
  after a `--force` run and asserted the lines were distinct, which was
  true because the file then held only the new batch. It is replaced by
  four tests that offer the first run's puzzles to the second, check
  both runs survive in the manifest and the flat file, refuse a mixed
  geometry, and stop when the directory and its manifest disagree.

- **G3 — `--count 0` succeeded silently.** It now fails with a remedy,
  because nobody asks for nothing on purpose and a silent success hides
  the typo that produced it.
