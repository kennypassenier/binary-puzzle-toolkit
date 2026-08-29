# Debugging guide

What to look at when something is wrong, and what the toolkit already
tells you before you have to look.

## The evidence trail

Every run leaves more than its exit code.

| Where | What it holds |
|---|---|
| exit code | 0 fine · 1 something did not succeed · 2 usage or file error · 3 cancelled |
| stderr | shortfalls, progress on a terminal, and every error with a remedy |
| `--explain` | the reasoning step by step: which strategy fired on which cell, every guess and every backtrack |
| `manifest.json` | per puzzle: the seed, index and attempt that regenerate it, its level, clue count, fingerprint, and how often the search budget ran out |
| `bpt --version` | the release plus the git revision it was built from, `-dirty` if the tree had uncommitted changes |

The manifest is the important one. A generated puzzle is never a
mystery: three numbers rebuild it exactly, so any question about it can
be asked again in isolation.

## Symptom to cause

### Solving

| Symptom | Likely cause |
|---|---|
| `#invalid:<line>` | the line did not parse. Run it alone to see the reason: wrong length, an odd size, a character that is not `0`, `1` or `.`, or a tag the format does not know |
| `#multiple:<line>` | the puzzle genuinely has more than one solution — with `--unique` that is an answer, not a failure |
| `#contradiction:<line>` | the givens already break a rule, or no filling can satisfy them |
| `#stuck:<line>` | only with `--no-backtrack`: human strategies ran out. Without that flag the search would have finished it |
| a composite solved as a plain grid | the tag was dropped. A tagless line of the right length parses as a plain n×n and simply does not enforce the inner regions |
| `#invalid` on a line that looks fine, with `--geometry` | the geometry describes a different grid size than the puzzle |

### Generating

| Symptom | Likely cause |
|---|---|
| `has no solution at all — it is over-constrained` | the geometry itself is impossible; no seed will help. `bpt inspect` on it usually names the reason |
| `region N is AxB: it has C columns of length D, but only E distinct balanced lines of that length exist` | a region too flat to satisfy its own unique-lines rule. There are only C(h, h/2) balanced lines of height h |
| `every attempt reproduced a puzzle already generated` | the space of distinct puzzles for that geometry is exhausted — expected on a 4x4, surprising on anything larger |
| `already holds N puzzle(s) — a batch owns its directory` | use `--force` to add to it, or pick an empty directory |
| `lists <file> but that file is gone` | the directory and its manifest disagree; nothing was added. Restore the file or start a fresh directory |
| `one directory holds one geometry at one level` | `--force` into a directory generated with a different `--kind` or `--level` |
| a run takes far longer than a sibling | ordinary: cost varies enormously between seeds. Measured on a 12x12, a median of 20 ms against a p95 of 1 second |
| `budget_hits` above zero in the manifest | the uniqueness search hit its bound, so the puzzle keeps clues a longer search might have removed. Valid, just not minimal |

## Reproducing exactly

Generation is fully determined by its inputs. To get one puzzle back:

```
bpt forge --kind <kind from the manifest> --seed <entry seed> --count <index+1>
```

and take the last line. The manifest records `attempt` too — above zero
means that puzzle collided with an earlier one and re-rolled, which the
sequence reproduces on its own.

A whole batch comes back from its manifest alone, and the restore drill
in `bpt/tests/restore_drill.rs` does exactly that against a committed
fixture on every CI run.

## When a change makes things slower

`benchmarks/baseline.json` holds a measured median and p95 per geometry.
CI re-measures the affordable ones and fails when the p95 drifts too
far. To re-record deliberately:

```
cargo test --release -p bpt-forge --test baseline -- --ignored --nocapture record_the_baseline
```

Each sample gets 150 seconds; a sample that runs past it is recorded as
unfinished rather than being waited on.

## Things that look like bugs and are not

- **The same seed gives different puzzles than it did before a release.**
  Reproducibility is promised for the same seed *and the same version*.
  A change to how the grid is filled or carved changes every seed.
- **A `--force` run's `completed` is lower than the puzzle count.**
  `completed` counts the run that last wrote the manifest; `puzzles`
  covers the whole directory.
- **`--level L3` produces L2 puzzles.** The level is a ceiling. Carving
  stops when removing more would push the puzzle past it, and sometimes
  it stops earlier.
