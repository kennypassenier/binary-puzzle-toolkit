# Feature ID map

binsolve and binforge each numbered their features independently, so
every ID collided when they merged. The solver keeps its numbers — it is
the larger half and nearly released — and the generator's shifted into a
free range. Every ID in this project now means exactly one thing.

This table exists because the generator's commits and test names from
before the merge still carry the old numbers. A commit saying `[K3]`
dated before 2026-08-28 in the generator means what is now **K22**.

## Generator features

| Was | Now | Feature |
|---|---|---|
| K1 | K20 | solution filler |
| K2 | K21 | carve loop with per-step uniqueness proof |
| K3 (K3a–e) | K22 (K22a–e) | the five special types |
| K4 | K23 | data-driven geometry model |
| K5 | K24 | difficulty measurement, four levels |
| K6 | K25 | difficulty targeting |
| K7 | K26 | dot-format output, puzzles + solutions |
| K8 | K27 | batch generation with corpus-style layout |
| K9 | K28 | two invented types, end-to-end |
| K10 | K29 | the generator CLI |
| K11 | K30 | benchmark harness |
| K12 | K31 | independent validation through the solver |
| M1–M11 | M20–M30 | in the same order |
| AR1–AR13 | AR20–AR32 | in the same order |

## Solver features

Unchanged: K1–K16, M1–M7, AR1–AR13, T1–T12.

## Reading a document

`docs/solve/` holds the solver's phase documents, `docs/forge/` the
generator's. Both are renumbered consistently, so an ID is unambiguous
wherever you meet it.
