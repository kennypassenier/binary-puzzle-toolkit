# binsolve

A complete solver for binary puzzles (Takuzu / Binairo) in Rust: every
standard n×n size, all five composite types from binarypuzzle.com, and
a uniqueness proof for each solution.

Status: **in development.** The solver, CLI and TUI are built and
tested; hardening, documentation and release (procedure phases 7–10)
are still open. There is no published binary yet — build from source.

## Build and run

```
cargo build --release
./target/release/binsolve "1..0....00.1.00..1......00.1...1..00"
```

On a terminal you get the grid plus statistics; piped or redirected you
get one canonical line per puzzle, so it composes with other tools:

```
binsolve --file puzzles.txt --out solutions.txt
binsolve --explain "1..0..."          # solving steps to stderr
binsolve --unique "1..0..."           # prove the solution is the only one
binsolve --check --file puzzle.txt    # verify a puzzle+solution file
```

Special types carry a tag: `4x6x6`, `4x8x8`, `9x6x6`, `8in14`,
`6in10in14` — for example `4x8x8:110..0.1…`.

Watch a puzzle being solved step by step:

```
cargo run --release -p binsolve-tui -- "1..0....00.1.00..1......00.1...1..00"
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

## License

MIT — see [LICENSE](LICENSE).
