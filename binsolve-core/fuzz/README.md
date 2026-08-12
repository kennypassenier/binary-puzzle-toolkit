# Fuzzing (M7)

Two coverage-guided targets, run on demand — not in per-push CI.
Requires the nightly toolchain (cargo-fuzz builds with libFuzzer):

```bash
rustup toolchain install nightly
cargo install cargo-fuzz
```

Run from `binsolve-core/`:

```bash
cargo +nightly fuzz run parse -- -max_total_time=3600
cargo +nightly fuzz run solve -- -max_total_time=3600
```

## Targets

| Target | Property asserted |
|---|---|
| `parse` | arbitrary bytes never panic; anything accepted round-trips through `serialize` unchanged |
| `solve` | the solver terminates on any parseable grid; every `Solved` outcome satisfies all rules and preserves the givens; `MultipleSolutions` returns two distinct grids |

Both targets bound their input (parse: 4 KiB; solve: grids up to 10×10)
so the fuzzer spends its time on logic rather than on memory pressure
or on the exponential blow-up that large near-empty grids have by
design.

## Findings

Every crash becomes a regression test **before** its fix (standing
rule 8). Findings so far:

| Date | Target | Input | Bug | Regression test |
|---|---|---|---|---|
| 2026-08-12 | `solve` | `.10.` | The strategy ladder reported `Solved` for any grid it filled completely, without checking global rules; this 2×2 forces both rows to `01`, breaking row uniqueness. | `k6_ladder_completion_must_still_satisfy_every_rule`, `k6_solved_outcomes_are_always_valid` |

Crashing inputs are minimized with `cargo +nightly fuzz tmin <target>
<artifact>` and then deleted once the regression test exists — the
test, not the artifact file, is the permanent record.
