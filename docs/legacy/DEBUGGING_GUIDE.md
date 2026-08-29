# Debugging guide

Where the evidence lives when something looks wrong, and what each
symptom usually means. Every message quoted here was produced by the
release binary, not written from memory.

## The evidence trail

binsolve tells you what it did on four separate channels, each with a
different audience:

| Channel | What it carries | How to get it |
|---|---|---|
| stdout | the canonical result, one line per puzzle | always |
| stdout (terminal only) | the grid plus statistics, above the canonical line | run it in a terminal, one puzzle, no `--out` |
| stderr | the numbered solving steps | `--explain` |
| a file | the same steps, kept | `--explain=FILE` |

The split matters: piping or redirecting stdout gives you only canonical
lines, so a script never has to parse prose. The trace never lands on
stdout, exactly so that the 1:1 line mapping survives.

Reading a trace:

```
step 1: FindDuo — r1c1 = 1 (cells r1c2 and r1c3 are both 0, neighbours must differ)
step 3: FillByCount — r1c0 = 0 (row 1 already has its 3 1s, the rest get the opposite)
step 6: AvoidTriple — r2c4 = 0 (cells r2c3 and r2c5 are both 1, the cell between must differ)
```

Each step names the strategy, the cell it filled, and why. When the
solver has to guess, the trace says so — `guess`, then `backtrack` when
that branch is refuted — so a long run of backtracks tells you the
puzzle is genuinely hard rather than that something is broken.

## Symptom → cause → what to do

### The command refuses to run

| Message | Cause | Do this |
|---|---|---|
| `no puzzle given — pass a puzzle string, or --file FILE …` | no argument at all | pass a puzzle, or `--file` |
| `cannot read <path> — check the path and permissions` | the batch file is missing or unreadable | check the path; the OS error is appended |
| `--check needs --file FILE with a puzzle line and a 'solution:' line` | `--check` used without a file | `--check` verifies files, not bare grids |
| `input file is empty — add one puzzle per line` | the file has no lines at all | check you wrote what you think you wrote |

All of these exit with code `2` — a usage problem, not a puzzle problem.

### A puzzle line comes back with `#invalid:`

The line never parsed. Run it with `--explain` and the reason appears on
stderr:

| Message | Cause | Do this |
|---|---|---|
| `invalid character 'x' at position 4 — a puzzle contains only '0', '1' and '.'` | a stray character, often a space or a letter from a bad scrape | clean the line; the position is 0-based |
| `grid has 35 cells, which is not a square of an even size` | the line lost or gained cells | count the characters; a 6×6 is 36, an 8×8 is 64 |
| `grid is 5x5, but binary puzzles need an even side length` | the length is a square, but of an odd number | the balance rule cannot hold on odd lines |
| `unknown puzzle type tag '7x7x7' — use one of 4x6x6, 4x8x8, 9x6x6, 8in14, 6in10in14` | a typo in the tag, or a type binsolve does not know | use one of the five, or drop the tag |
| `tag '4x8x8' requires exactly 256 cells but the grid has 64` | tag and grid disagree | fix whichever is wrong; the tag decides the size |
| `line is empty` | a blank line in the batch | blank lines keep their slot on purpose, so the line numbering stays true; remove it if you did not intend it |

### A puzzle comes back with `#contradiction:`

The puzzle has no solution. The terminal output names the specific rule:

- *"row 4 has three consecutive 0s starting at position 2"* — a triple
  among the givens.
- *"column 3 already has 4 0s but may hold at most 3"* — more of one
  value than the line can take.
- *"rows 0 and 1 … are identical"* — a duplicate line.
- *"cell r0c2 is forced to both 0 and 1 by different rules"* — two sound
  deductions disagree, so the givens are inconsistent.
- *"no assignment of the open cells satisfies all rules (search
  exhausted)"* — nothing locally wrong, but the search refuted every
  possibility.

For a scraped puzzle this almost always means a cell was misread. Compare
against the source.

### A puzzle comes back with `#multiple:`

More than one solution exists. A published puzzle never has that, so
the scrape probably lost a given. Use `--explain` to see how far the
forced reasoning got before the solver had to guess.

### A puzzle comes back with `#stuck:`

Only possible with `--no-backtrack`, which forbids guessing. The human
strategies ran out. On a terminal you also see the partial grid and the
percentage filled. Without the flag, the search would finish the job.

### The output has fewer lines than the input

It should not: every input line produces exactly one output line, blank
lines included. If you see a mismatch, that is a bug worth reporting —
the case that used to cause it (silently dropping blank lines) is now
pinned by `k9_blank_lines_keep_the_line_mapping` in
`bpt/tests/cli.rs`.

### The terminal shows a grid where a script expects one line

The grid and statistics appear only when stdout is a terminal, a single
puzzle was given, and `--out` was not used. If a pipe is receiving them,
that is a regression against the promise that pipes get canonical lines
only.

### Writing the output file failed

Output is atomic: a failure leaves the previous file intact rather than a
truncated one. If another program holds the destination open (most likely
on Windows), close it and retry. A leftover `.<name>.tmp` next to the
destination is safe to delete; see the runbook, procedure 9.

## When you suspect the solver itself

1. Reproduce with `--explain` and keep the trace.
2. Check whether the answer is *wrong* or merely *unexpected*: run
   `--check` against a known solution, or `--unique` to see whether the
   puzzle is ambiguous.
3. Every solution the solver reports has already been validated against
   every rule of every region before you see it, so a rule-breaking grid
   would be a serious bug — capture the input.
4. Traces are reproducible: the same puzzle always produces the same
   trace, byte for byte. If two runs differ, that itself is the bug.
