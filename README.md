# BinaryPuzzleToolkit

One toolkit for binary puzzles (Takuzu / Binairo) in Rust. It **solves**
them — every standard n×n size, all five composite types from
binarypuzzle.com, and a uniqueness proof for each solution — and it
**generates** them, including invented geometries defined in a file.

Merged on 2026-08-28 from two projects that were designed for each
other: the solver (binsolve) and the generator (binforge).

Status: **in development.** The solving half is built, tested,
hardened and documented; the generating half has its model and geometry
in place and its generation pipeline still to build. There is no published binary
yet — build from source. Windows is build-verified (CI compiles and
runs the suite there) but not yet runtime-verified on real hardware:
see [docs/solve/WINDOWS_TEST_CHECKLIST.md](docs/solve/WINDOWS_TEST_CHECKLIST.md).

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
`6in10in14` — for example `4x8x8:110..0.1…`.

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
| [docs/solve/USER_GUIDE.md](docs/solve/USER_GUIDE.md) | every command and flag, with worked examples |
| [docs/solve/OPERATIONS_RUNBOOK.md](docs/solve/OPERATIONS_RUNBOOK.md) | numbered procedures: batches, verification, fuzzing, recovery |
| [docs/solve/DEBUGGING_GUIDE.md](docs/solve/DEBUGGING_GUIDE.md) | symptom → cause → remedy for every error message |
| [docs/solve/ARCHITECTURE_REFERENCE.md](docs/solve/ARCHITECTURE_REFERENCE.md) | how the solver works, as built |
| [docs/solve/TEST_PLAN.md](docs/solve/TEST_PLAN.md) | what is proven where, and what is deliberately not |
| [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) | working on the code; the one-time hook activation |

## License

MIT — see [LICENSE](LICENSE).
