# BinaryPuzzleToolkit

One toolkit for binary puzzles (Takuzu / Binairo) in Rust. It **solves**
them — every standard n×n size, all five composite types from
binarypuzzle.com, and a uniqueness proof for each solution — and it
**generates** them, including invented geometries defined in a file.

Merged on 2026-08-28 from two projects that were designed for each
other: the solver (binsolve) and the generator (binforge).

Status: **both halves are built, tested and hardened.** Solving and
generating are complete, including invented composite types, batches
with a manifest that can rebuild them, and independent validation of
every generated puzzle through the solver. There is no published binary
yet — build from source.

Windows is **build-verified, not runtime-verified**: CI compiles and
runs the whole suite there, and it has caught real failures, but nobody
has started the toolkit on a real Windows machine. By the project's own
rule that is beta until
[the checklist](docs/solve/WINDOWS_TEST_CHECKLIST.md) is signed.
[docs/TEST_PLAN.md](docs/TEST_PLAN.md) lists what else is deliberately
not covered.

## Build and run

```
cargo build --release
./target/release/bpt solve "1..0....00.1.00..1......00.1...1..00"
```

On a terminal you get the grid plus statistics; piped or redirected you
get one canonical line per puzzle, so it composes with other tools:

```
bpt solve --file puzzles.txt --out solutions.txt
bpt solve --explain "1..0..."          # solving steps to stderr
bpt solve --unique "1..0..."           # prove the solution is the only one
bpt solve --check --file puzzle.txt    # verify a puzzle+solution file
```

Special types carry a tag: `4x6x6`, `4x8x8`, `9x6x6`, `8in14`,
`6in10in14` — for example `4x8x8:110..0.1…`. A tag describes its own
layout, so a type nobody has generated before works the moment you name
it: `4x6x6in16` is four 6x6 blocks forming a 12x12, centred in a 16x16.

Generate puzzles, each proven to have exactly one solution:

```
bpt forge --kind 10 --count 5                   # five 10x10 puzzles
bpt forge --kind 4x6x6in16 --level L2 --count 3 # an invented type
bpt forge --kind 12 --count 100 --out-dir corpus/generated
```

A batch writes one file per puzzle, a flat file the solver can validate
in one run, and a manifest that records the three numbers rebuilding
each puzzle. Inspect any layout before generating from it:

```
bpt inspect 4x6x6in16
```

Watch a puzzle being solved step by step:

```
bpt watch "1..0....00.1.00..1......00.1...1..00"
```

## Contributing / working on this repo

**Run this once after cloning** — it activates the commit gates:

```
git config core.hooksPath .githooks
```

Without it, nothing stops a commit that fails formatting, lint or the
test suite. Git cannot carry this setting inside a clone, so it is a
manual step for every checkout. See
[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for the full development
setup, the quality gates and how to run the fuzzers.

## Documentation

| Document | For |
|---|---|
| [docs/USER_GUIDE.md](docs/USER_GUIDE.md) | every command and flag, with worked examples |
| [docs/OPERATIONS_RUNBOOK.md](docs/OPERATIONS_RUNBOOK.md) | numbered procedures: install, update, generate, restore, release |
| [docs/DEBUGGING_GUIDE.md](docs/DEBUGGING_GUIDE.md) | the evidence a run leaves, and symptom → cause |
| [docs/ARCHITECTURE_REFERENCE.md](docs/ARCHITECTURE_REFERENCE.md) | how the toolkit works, as built |
| [docs/TEST_PLAN.md](docs/TEST_PLAN.md) | what is proven where, and what is deliberately not |
| [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) | working on the code; the one-time hook activation |

The solver half's own documents from before the merge are kept in
[docs/legacy/](docs/legacy/); where they disagree with the documents
above, the ones above are current.

## License

MIT — see [LICENSE](LICENSE).
