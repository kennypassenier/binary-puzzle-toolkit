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
```

The `solve` target is run in chunks, because libFuzzer's own
instrumentation state grows past 6 GB after a few million executions
on this target (measured: the solver itself peaks at 36 MB over 50,000
consecutive solves in one process, so the growth is the harness, not
the code). The corpus persists in `fuzz/corpus/solve/`, so coverage
still accumulates across chunks:

```bash
for i in 1 2 3 4; do
  cargo +nightly fuzz run solve -- -max_total_time=900 -rss_limit_mb=6144 -timeout=60
done
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
| 2026-08-12 | `solve` | sparse 10×10 (slow unit) | DFS explored branches that had already over-filled a line; strategies act only on empty cells, so nothing pruned them. Validating the partial grid at each node: 2.08 s → 1 ms. | `k13_adversarial_sparse_inputs_stay_fast` |

## Clean runs on the fixed code

| Date | Target | Executions | Wall time | Result |
|---|---|---|---|---|
| 2026-08-12 | `parse` | 1,592,308,726 | 1 h | no findings |
| 2026-08-28 | `solve` | 7,562,737 (4 × 15 min) | 1 h | no crashes; one slow unit at 0.47 s native (inside the 1 s G5 bound), pinned as a regression case |

Crashing inputs are minimized with `cargo +nightly fuzz tmin <target>
<artifact>` and then deleted once the regression test exists — the
test, not the artifact file, is the permanent record.
