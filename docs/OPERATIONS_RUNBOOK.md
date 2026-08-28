# Operations runbook

Numbered procedures for running binsolve and for keeping the project
healthy. Every command here was executed against the release binary
before it was written down.

## 1 · Solve a batch of scraped puzzles

1. Put one puzzle per line in a text file. Special types carry their
   tag, for example `4x8x8:110..0.1…`; plain puzzles need no tag.
2. Run the solver, writing results to a second file:
   ```
   binsolve --file puzzles.txt --out solutions.txt
   ```
3. Check the exit code: `0` every puzzle solved, `1` at least one did
   not, `2` the command or the file was unusable.
4. If the exit code is `1`, find the failures — they are the lines
   starting with `#`:
   ```
   grep -n '^#' solutions.txt
   ```
   Output line N always describes input line N, so the line number is
   the puzzle's position in the input.

Example of a mixed run:

```
$ binsolve --file puzzles.txt
101010010011100101011010001101110100
#contradiction:000.................................
#invalid:rommel
$ echo $?
1
```

## 2 · Verify a scraped puzzle whose answer you do not have

The uniqueness proof replaces a published solution: exactly one solution
means the answer is verified.

1. Run with `--unique`:
   ```
   binsolve --unique "1..0....00.1.00..1......00.1...1..00"
   ```
2. A grid on stdout means the solution is proven unique — that grid *is*
   the verified answer.
3. `#multiple:` means the puzzle has more than one solution. A published
   puzzle never does, so this indicates a bad scrape: givens were lost
   or misread.
4. `#contradiction:` means no solution exists at all — the scrape
   corrupted a cell rather than dropping one.

```
$ binsolve --unique "................"
#multiple:................
$ echo $?
1
```

## 3 · Check a puzzle against a known solution

For corpus files that carry a `solution:` line:

```
$ binsolve --check --file corpus/standard/6/bp-s6l1n1-20260812-easy.txt
ok: corpus/standard/6/bp-s6l1n1-20260812-easy.txt
$ echo $?
0
```

A mismatch prints one `invalid:` line per violated rule, naming the rule
and the location, and exits `1`.

## 4 · Add a puzzle to the test corpus

1. Create `corpus/{standard|special}/<size-or-tag>/bp-<id>-<date>-<difficulty>.txt`.
2. Line 1: the puzzle in the normal one-line format, tag included for
   special types. Line 2: `solution:` followed by the full grid.
3. Verify it before committing:
   ```
   binsolve --check --file <the new file>
   ```
4. Update the corpus count in `binsolve-core/tests/solve.rs` — the test
   asserts an exact inventory rather than a floor, so that deleting
   fixtures cannot go unnoticed.
5. Run `cargo test --workspace`; the corpus meta-test will parse the new
   file and validate its solution against every region rule.

## 5 · Investigate why a puzzle will not solve

1. Ask the solver to explain itself:
   ```
   binsolve --explain "<puzzle>" 2> trace.txt
   ```
2. Read the last steps of `trace.txt`. Each line names the strategy, the
   cell and the reason.
3. To see how far human strategies get without guessing:
   ```
   binsolve --no-backtrack "<puzzle>"
   ```
   `#stuck:` means the strategies ran out; on a terminal you also get the
   partially filled grid and the percentage reached.
4. To watch it happen step by step, use the TUI:
   ```
   binsolve-tui "<puzzle>"
   ```

## 6 · Run the performance check

The speed promises assert only in release builds:

```
cargo test --release -p binsolve-core --test thresholds -- --nocapture
```

This prints the worst single puzzle, the median, and the 1,000-puzzle
batch time, then asserts them against the targets. CI runs the same job
on Linux and Windows on every push.

## 7 · Run the fuzzers

Requires the nightly toolchain and cargo-fuzz (see
[DEVELOPMENT.md](DEVELOPMENT.md)). The solver target is run in chunks
because libFuzzer's own memory grows over long runs:

```
cd binsolve-core
cargo +nightly fuzz run parse -- -max_total_time=3600
for i in 1 2 3 4; do
  cargo +nightly fuzz run solve -- -max_total_time=900 -rss_limit_mb=6144 -timeout=60
done
```

Any crash artefact becomes a regression test **before** it is fixed.
Record the finding in `binsolve-core/fuzz/README.md`.

## 8 · Restore the commit gates after cloning

Git cannot carry this setting inside a clone:

```
git config core.hooksPath .githooks
git config core.hooksPath      # must print .githooks
```

Without it, commits are not gated at all. See
[DEVELOPMENT.md](DEVELOPMENT.md) for what the gates check.

## 9 · Recover a failed write

Output is written atomically, so a failure leaves the previous file
intact rather than a truncated one.

1. If the error names a file that another program holds open (most
   likely on Windows), close that program and run the command again.
2. Look for a leftover temporary file next to the destination — it is
   named `.<destination>.tmp`. Removing it is safe; nothing reads it.
3. There is no automatic cleanup of such a file yet; this is a known
   open item recorded in [TEST_PLAN.md](TEST_PLAN.md).
