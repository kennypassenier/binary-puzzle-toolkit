# Operations runbook

Numbered procedures. Each one is written to be followed without knowing
how the toolkit works inside.

## 1 · Install from a clean machine

1. Install Rust (rustup). The toolchain is pinned in
   `rust-toolchain.toml`, so no version choice is needed.
2. `git clone https://github.com/kennypassenier/binary-puzzle-toolkit`
3. `cd binary-puzzle-toolkit`
4. `git config core.hooksPath .githooks` — once per clone. Without it
   the format, lint and test gates do not run before a commit.
5. `cargo build --release`
6. The binaries are `target/release/bpt` and `target/release/bpt-tui`.
   Put them on your PATH, or run them from there.

Verify: `bpt --version` prints the release and the git revision.

## 2 · Update an installed copy

1. `cd binary-puzzle-toolkit`
2. `git pull`
3. `cargo build --release`

There is no self-update and no package. This is deliberate: one user,
one machine, and a step that cannot go wrong halfway.

Note that a new version may generate different puzzles for the same
seed. Reproducibility is promised for the same seed *and* the same
version; `bpt --version` records which one made a batch, and so does
every manifest.

## 3 · Generate a corpus

1. Pick a directory that does not exist yet, or is empty.
2. `bpt forge --kind <type> --level <L1..L4> --count <n> --seed <seed> --out-dir <dir>`
3. Check the exit code: 0 means every puzzle asked for was produced.
   1 means fewer — stderr says which and why.
4. Validate independently:
   `bpt solve --file <dir>/puzzles.txt --unique`
   Exit 0 with no `#multiple:` line means every puzzle has exactly one
   solution, checked by the solver rather than by the generator.

To add to an existing corpus later, repeat step 2 with `--force` and a
different seed. The run refuses any puzzle already there, and the
manifest and `puzzles.txt` grow to cover everything in the directory.

## 4 · Restore a corpus from its manifest

The manifest is the backup: it holds the three numbers that rebuild each
puzzle. Nothing else needs backing up, because the repository holds the
scraped corpus and the code.

1. `git clone` the repository into an empty directory.
2. `cargo build --release`
3. `cargo test --release -p bpt --test restore_drill`

That regenerates a batch committed months earlier from its manifest
alone and compares it byte for byte. If it passes, reproducibility still
holds; if it fails, something about generation changed and the message
says which file disagreed.

To rebuild a specific batch rather than the fixture, read `kind`,
`grid_size` and `level_ceiling` from its manifest and re-run procedure 3
with the seed of each entry.

## 5 · Re-record the performance baseline

Do this when a deliberate change makes generation faster or slower, and
never to make a failing check pass without looking at why.

1. `cargo test --release -p bpt-forge --test baseline -- --ignored --nocapture record_the_baseline > /tmp/baseline.out`
2. Take the JSON array from that output into `benchmarks/baseline.json`.
3. `cargo test --release -p bpt-forge --test baseline` to confirm the
   guard passes against the new numbers.
4. Commit with a message saying what changed and why the numbers moved.

## 6 · Check a release before tagging

1. `cargo test --workspace` — everything green.
2. `cargo test --release -p bpt --test validation -- --ignored` — every
   geometry validated in bulk through the binary.
3. `cargo test --release -p bpt-forge --test thresholds` — the speed
   promises.
4. Confirm CI is green on the commit being tagged.
5. `bpt --version` shows the intended release and a revision without
   `-dirty`.

## 7 · What to do when a generation run will not finish

Generation is bounded and always terminates, but "terminates" is not
"quickly": a single 16x16 has been measured at fifteen minutes.

1. Ctrl-C once. The run finishes the puzzle in flight, writes what is
   done with `status: cancelled`, and exits 3. What landed is complete
   and valid.
2. Ctrl-C again aborts immediately if the first does not return.
3. To go faster: lower `--level`, or generate a smaller size. Symmetry
   costs time as well as clues.
