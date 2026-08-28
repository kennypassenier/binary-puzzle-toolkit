# Development guide

## One-time setup after cloning

```
git config core.hooksPath .githooks
```

This is **required** and cannot be carried inside a clone: `git` stores
`core.hooksPath` in the local config, which is not part of the
repository. Until you run it, commits are not gated — formatting, lint
and test failures pass straight through, and commit messages are not
checked for feature IDs.

Verify it took effect:

```
git config core.hooksPath      # must print .githooks
```

## What the gates do

Two layers, both enforcing the same rules:

| Layer | Scope | Runs |
|---|---|---|
| `.githooks/pre-commit` | any terminal, any tool | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace` |
| `.githooks/commit-msg` | any terminal, any tool | message must carry feature IDs in brackets, e.g. `[K5, AR9]` or `[meta]` |
| `.claude/hooks/check-commit.sh` | Claude Code sessions opened in this directory | the same two checks, as a second layer |
| GitHub Actions (`ci.yml`) | every push and pull request | the same gates on Ubuntu, Windows and an Arch container, plus a check that the solver core has no runtime dependencies |

`main` is protected: all four CI checks must pass, and force-pushes and
branch deletion are refused.

## Running the tests

```
cargo test --workspace              # full suite
cargo test --release --workspace    # also asserts the performance targets
```

Performance thresholds only assert in release builds; a debug build is
10–30× slower and would fail meaninglessly, so the test prints its
measurements and skips the assertions there.

## Benchmarks

```
cargo bench
```

Criterion benchmarks cover per-puzzle solving, uniqueness proving and a
1,000-puzzle batch.

## Fuzzing

Requires the nightly toolchain; see [../bpt-core/fuzz/README.md](../bpt-core/fuzz/README.md)
for the targets, the chunked-run workaround and the findings so far.

```
rustup toolchain install nightly
cargo install cargo-fuzz
cd binsolve-core && cargo +nightly fuzz run parse -- -max_total_time=3600
```

## Project conventions

This project follows the development procedure in
`~/Projects/dev-procedure`. Practical consequences for a contributor:

- Every commit message names the feature IDs it implements; see
  [solve/FEATURES.md](solve/FEATURES.md) for what each ID means.
- Every bug fix is preceded by a test that fails without the fix.
- Frozen decisions (the feature list, the architecture) change only
  through a recorded amendment, not silently.
