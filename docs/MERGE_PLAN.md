# Merge plan — BinaryPuzzleToolkit

Decided 2026-08-28: binsolve and binforge become one project.

| Decision | Choice |
|---|---|
| Name | **BinaryPuzzleToolkit**; repo and workspace `binary-puzzle-toolkit`, command `bpt` |
| Feature IDs | one shared list; the generator's IDs are renumbered so every ID means one thing |
| Repository | rename the binsolve repo, pull binforge's history into it |
| Interface | one command with subcommands: `bpt solve`, `bpt forge`, `bpt watch` |

`bpt` follows the usual short-command convention (`ripgrep` → `rg`); the
full name stays the project and repository name.

## Target layout

```
bpt-core     grid, regions, rules, the text format, atomic writes
bpt-solve    strategies, search, events, difficulty grading
bpt-forge    geometry, fill, carve, grading of generated puzzles
bpt          one binary: solve / forge / watch subcommands
```

`bpt-core` is what the two projects duplicated: both defined their own
`Region` and `RuleSet`, and both had their own atomic write. The
generator's version of that write is the better one — it retries only on
a Windows sharing violation, where the solver's waits on any I/O error —
so the merged core takes binforge's implementation.

## ID renumbering

Both projects independently numbered K1…, M1…, AR1…, so every ID
collides. The solver keeps its numbers (it is the larger, nearly
released half); the generator's shift into a free range:

| Generator, was | Becomes |
|---|---|
| K1–K12 | K20–K31 |
| M1–M11 | M20–M30 |
| AR1–AR13 | AR20–AR32 |

A mapping table lands in the merged `docs/FEATURES.md` so that the
generator's existing commits and test names stay readable: a commit
saying `[K3]` from before the merge means the generator's K3, now K22.

## Order of work

1. Pull binforge's history into this repository, its files in a
   subdirectory. The workspace does not include it yet, so the build
   stays green.
2. Create `bpt-core` from the solver's grid/region/parse plus the
   generator's atomic write; make the solver crate use it.
3. Move the generator onto `bpt-core`, dropping its duplicate `Region`
   and `RuleSet`.
4. Merge the two CLIs into one binary with subcommands.
5. Renumber the generator's IDs and merge the phase documents.
6. Rename the repository and the local directory.
7. Update CI, hooks and branch protection for the new layout.

Each step ends with the full gates green before the next begins.
