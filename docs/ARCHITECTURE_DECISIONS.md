# Architecture Decisions — binforge

Phase 3 entries decided 2026-08-12 (tech-choice gate, all items as
recommended). Phase 4 entries follow. Changes only via mini-rounds
(FORM_PROTOCOL §5).

## Phase 3 · Tech choice

| ID | Decision | Choice |
|---|---|---|
| T1 | Toolchain, edition, MSRV | rustup + `rust-toolchain.toml` pin; edition 2024; no MSRV (track stable) — identical to binsolve T1/T2/T12 |
| T2 | Workspace layout | two crates: `binforge-core` (lib, zero ambient I/O) + `binforge` (CLI: clap + anyhow) |
| T3 | binsolve-core dependency | git dependency pinned to a commit (`https://github.com/kennypassenier/binsolve.git`, `rev = …`); local path override allowed for co-development |
| T4 | RNG | `rand` + `rand_chacha` (`ChaCha8Rng`): reproducible across versions/platforms, u64-seedable, per-worker stream derivation for M3 |
| T5 | Geometry definitions (K4) | TOML via serde — comment-friendly, hand-editable |
| T6 | Manifest format (M4) | JSON via serde_json — program-written, `jq`-greppable, feeds M10 |
| T7 | Parallelism (M3) | rayon — input-order collection keeps parallel-equals-sequential honest |
| T8 | Duplicate detection (M2) | `HashSet<String>` of puzzle lines, zero deps — exact, no collision reasoning |
| T9 | Progress + cancellation (M7) | hand-rolled stderr progress line (silent when not a TTY) + `ctrlc` crate for portable Linux/Windows signal handling |
| T10 | Testing / benchmarking | proptest (M9, shrinking), insta (M6 ASCII + CLI help snapshots), criterion (K11) + a release-mode test asserting hard G8 thresholds |
| T11 | Dependency policy | strict allowlist; new dependency ⇒ mini-round. Runtime: binsolve-core, clap, thiserror, anyhow, rand, rand_chacha, serde, toml, serde_json, rayon, ctrlc. Dev: proptest, insta, criterion. (Zero-dep core is not achievable here — RNG and geometry parsing live in core — so the allowlist is the discipline.) |
| T12 | License | MIT — matches binsolve |

## Phase 4 · Architecture

*(in progress — draft under adversarial review)*
