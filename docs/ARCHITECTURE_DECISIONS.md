# Architecture Decisions — binsolve

Phase 3 entries decided 2026-08-12 (tech-choice gate, all items as
recommended). Phase 4 entries follow after the architecture gate.

## Phase 3 · Tech choice

| ID | Decision | Choice |
|---|---|---|
| T1 | Toolchain management | rustup + `rust-toolchain.toml` pin (nightly available for T9) |
| T2 | Rust edition | 2024 |
| T3 | CLI parsing | clap (derive) |
| T4 | TUI stack | ratatui + crossterm |
| T5 | Error handling | thiserror (library), anyhow (binaries) |
| T6 | Property testing | proptest (dev-dep) |
| T7 | Snapshot testing | insta (dev-dep) |
| T8 | Benchmarking | criterion (dev-dep); hard G5 thresholds also in a release-mode test |
| T9 | Fuzzing | cargo-fuzz / libFuzzer, nightly, Linux, on-demand |
| T10 | Dependency policy | strict allowlist: runtime clap, ratatui, crossterm, thiserror, anyhow (+rayon reserved for M5); dev proptest, insta, criterion, cargo-fuzz. **Solver core: zero dependencies (pure std).** New dep ⇒ mini-round |
| T11 | License | MIT |
| T12 | MSRV | none — track current stable via the T1 pin |
