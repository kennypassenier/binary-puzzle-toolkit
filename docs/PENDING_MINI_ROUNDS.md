# Pending mini-rounds (AFK queue)

**Q1, Q2, Q4 and the enforcement items were resolved at the combined
AFK gate on 2026-08-28.** The commit hook is synced with the procedure
template (Q1/Q2), and the `parse.rs` comment (Q4) is corrected below.
Q3 — the three mandatory Phase-2 items — is being run as its own round.


Deviations found while Kenny was away. Per the AFK rule these are NOT
built silently: the affected area is quarantined, the deviation is
queued here, and work continues on everything unaffected. These are the
first thing presented when Kenny returns.

## Q1 · The Claude Code commit hook has a stale ID regex

**Area quarantined:** `.claude/hooks/check-commit.sh`.

**What happened.** The project's copy dates from 2026-08-12 and uses
`\[(meta|[A-Za-z]{1,4}[0-9][^]]*)\]`, which requires the closing bracket
immediately after the ID. The procedure's own template was fixed on
2026-08-28 to `\[(meta|[A-Za-z]{1,4}[0-9])[^]]*\]`. Consequence: a
correct message like `[meta, M7]` is REJECTED by the project hook while
the git-native `commit-msg` hook and CI both accept it. Every commit
during this AFK stretch therefore carries a single ID.

**Why it is queued, not fixed.** Changing the enforcement machinery is
always a gate (procedure ground rules), even when the change is only a
sync to the approved upstream template.

**Options when Kenny returns:** sync the file from
`~/Projects/dev-procedure/hooks/check-commit.sh` (recommended) · leave
it and keep using single IDs · drop the Claude Code layer entirely now
that the git-native hooks cover every session.

## Q2 · The same hook matches "git commit" anywhere in a command

**Area quarantined:** `.claude/hooks/check-commit.sh`.

**What happened.** The hook greps the whole command string for
`git commit`. A script that merely CONTAINS that text — a heredoc
writing documentation about the flag, for instance — is treated as a
commit and blocked. It also cannot see the message when the real commit
uses `-F file` or `-F -`, so valid commits are rejected for "missing
IDs" that are in fact present.

**Impact.** False positives block unrelated work; false negatives are
possible too (the git-native `commit-msg` hook reads the actual message
file and is not fooled either way).

**Options:** tighten the match to a command that STARTS a commit and
read the message from `-m` / `-F` properly · rely on the git-native
layer alone for message checking · accept as a known quirk.

## Resolved by the merge, 2026-08-28

The five queued changes the generator was waiting on no longer needed a
cross-repository mini-round, because there is no longer a second
repository:

- **B1 choice oracle** — built: `solve_with` takes a `ChoiceOracle`, the
  default keeps the solver bit-deterministic.
- **B2 custom geometries** — built: `PuzzleKind::Custom` plus
  `Puzzle::custom`, so an invented layout solves like a known one.
- **B3 rectangular regions** — built: `Region` carries `rows`/`cols`
  with `Region::square` for the six known kinds.
- **B5 git revision in `--version`** — built (2026-08-29): a build
  script stamps the short hash, `-dirty` when the tree had uncommitted
  changes, `unknown` outside a checkout. Batch manifests record it, so a
  batch can always be traced to the build that wrote it.
- **B4 node budget** remains open; it blocks nothing today except the
  cancellation half of M26 (see Q5).

## Q3 · Phase 2's three mandatory items — DECIDED 2026-08-29

**Area quarantined:** none — these are additions, nothing built on them.

The procedure gained three mandatory Phase-2 discussion items after
binsolve's feature list was frozen, so they were never put to Kenny:

1. **Update & distribution mechanism** — how does an installed binsolve
   get updated? Built-in self-update, package manager, `git pull` +
   rebuild, or consciously none.
2. **Ecosystem integration** — binsolve and binforge are registered as a
   companion pair, but the shared puzzle-format contract has never been
   formally decided as a feature.
3. **Backup & restore** — what IS the state (the corpus? nothing else?),
   and does it need a backup at all.

**Decided 2026-08-29.**
1. **Update & distribution** — `git pull` + `cargo build --release`. Kenny
   is the only user and the repository is on his own machine; anything
   heavier is machinery for a problem he does not have. It becomes a
   numbered step in the operations runbook (Phase 8).
2. **Ecosystem integration** — the shared line format is pinned as a
   feature with regression vectors: a handful of fixed example lines with
   their expected interpretation, so a change to the format announces
   itself instead of breaking old corpus files.
3. **Backup & restore** — the repository is the backup. The scraped
   corpus lives in it, generated batches are reproducible from their
   manifest, and M30's drill proves that from a fresh clone in CI.

**On item 2, what already exists.** `bpt-core/tests/format_vectors.rs`
already pins the format against files: eight valid lines covering all
five published tags, each asserted to round-trip byte for byte, and
seven invalid ones asserted to fail with a specific error class. So the
mechanism is built; what is missing is that it was never *recorded* as a
feature, which is what this item asked for. That write-up waits on Q6:
if invented types gain a tag, the vectors need a case for it, and it
would be silly to freeze the feature text the day before.

## Q4 · A code comment argues something measurement contradicts — DECIDED 2026-08-29

**Area quarantined:** the comment in `bpt-core/src/parse.rs` that
justifies the integer-sqrt guard.

**What happened.** The comment reads "floating point may round either
way". Measured over every perfect square up to 4096²: the naive
`(len as f64).sqrt() as usize` is never wrong, so the stated reason does
not hold for any input this program can receive. The guard ITSELF is
correct — it agrees with the truth on all 200,000 lengths tested — only
its justification is an argument that was never executed.

**Options:** reword the comment to say what is actually true (the guard
is defensive, cheap, and makes the intent explicit) · drop the guard and
rely on the naive conversion · leave both as they are.

**Decided 2026-08-29: reword the comment.** The guard stays — it is
cheap and makes the intent explicit — but its stated reason describes a
danger that does not exist for this input, and a comment that misleads
the next reader is worse than no comment.

**Already in the code.** Checking before changing anything: the comment
at `bpt-core/src/parse.rs` now reads "Measured over every perfect square
up to 4096 the naive conversion is never wrong, so this is defensive
rather than load bearing". That is exactly what was decided, so the
decision needed no edit — only this note saying so.

## Q5 · Ctrl-C on a long batch — withdrawn, the decision was already made

**Resolved without a mini-round (2026-08-29).** This was queued as a
dependency question and should not have been: T9 already chose the
`ctrlc` crate for exactly this, and T11's allowlist names it. Nothing
was open. Cancellation is built: Ctrl-C finishes the puzzle in flight,
writes the batch with `status: cancelled`, and exits 3; a second Ctrl-C
aborts.
↳ T9 = the technology choice for M26 (progress + cancellation);
T11 = the dependency allowlist.

What genuinely remains is the *grain*: the smallest interruptible unit
is one puzzle, which on a 16x16 is seconds. Mini-round **B4**'s
deterministic node budget is what would make it finer, and B4 is
Kenny's own "later".

## Q6 · Invented types have no tag, and the format cannot grow one by itself

**Area quarantined:** the puzzle-line tag vocabulary.
↳ K28 = two invented types end-to-end; S6b = Kenny's frozen choice that
a special type is marked by a **prefix** on the line (`4x8x8:110...`).

**What is built.** `4x10x10` and `8in12in16` exist as geometry files
only — no code knows their names. Both `bpt forge` and `bpt solve` take
`--geometry FILE`, so the two halves agree on the regions and the types
generate and solve end to end.

**What is open.** They emit **no** prefix. A reader resolves a prefix
through a fixed vocabulary and rejects anything else, so writing
`4x10x10:` today would produce files nothing can read. That is why K28
made the tag a mini-round in the first place.

A test pins what is at stake: the same 8in12in16 puzzle read *without*
its geometry parses as a plain 16x16 and has more than one solution.
Nothing crashes, which is exactly why it must not stay implicit.

**Options:** register invented types in the built-in vocabulary, like
the five published ones · resolve an unknown prefix by looking for
`geometries/<tag>.toml`, which makes any invented type work without
code changes but gives the format a filesystem dependency · make the
prefix self-describing so the line carries its own regions · leave
`--geometry` as the only way, and never tag invented types.

## Q7 · Generation has no upper bound on the largest grids — DECIDED 2026-08-29

**Area quarantined:** none — this is a measurement. Nothing was built on
it, and nothing was quietly weakened because of it.
↳ G8 = the scope's performance targets for generation; AR25 = the
decision that replaced G8's figures with a measured baseline; B4 = the
deterministic node budget mini-round, which Kenny scheduled as "later";
D1 = 100 validated puzzles per geometry.

**What was measured** on Kenny's PC, release build, level L4:

| geometry | one puzzle |
|---|---|
| 14x14 | 3.9 s (G8 target: under 10 s — met) |
| 100x 10x10 | 0.7 s (G8 target: under 60 s — met) |
| 4x6x6 | 0.05 s |
| 4x8x8 | 5.5 s |
| **9x6x6** | **94.4 s** (G8 target: under 30 s — **missed by 3x**) |
| **18x18** | **2 s, 11 s, 84 s, or never** — see below |

The last row is the serious one. 18x18 is not *slow*; it is
**unbounded**, and not rarely. Five seeds, one puzzle each,
single-threaded, release:

| seed | 31 | 77 | 555 | 2026 | 909 |
|---|---|---|---|---|---|
| time | 11 s | 2 s | 84 s | not finished in 180 s | not finished in 180 s |

Two of five did not finish in three minutes, and seed 2026 was still
running after eighty. A separate debug-build run of an 18x18 was found
still going after 2.8 hours. This is precisely the failure AR26
predicted when it said a deterministic node budget was needed before
anything could promise not to hang.

**What was already done about it.** Two carve optimisations landed on
the way to these numbers — the uniqueness proof after each removal
became a refutation of the opposite value, and the grading call inside
the loop lost its search — together taking a 16x16 from 30.8 s to 5.5 s
with byte-identical output. They lowered the curve; they did not put a
ceiling on it, and no optimisation can.

**The recorded baseline**, level L4, seeds fixed, one thread, 150 s per
sample (`benchmarks/baseline.json`):

| geometry | samples finished | median | p95 | unfinished |
|---|---|---|---|---|
| 6x6 | 20 | 0.4 ms | 0.4 ms | — |
| 8x8 | 20 | 1.4 ms | 2.2 ms | — |
| 10x10 | 20 | 4.7 ms | 7.2 ms | — |
| 12x12 | 20 | 19.7 ms | 1.02 s | — |
| 14x14 | 10 | 146 ms | 3.45 s | — |
| 16x16 | 2 | 6.0 s | 9.2 s | 1 of 3 |
| 18x18 | 1 | 19.5 s | 19.5 s | 2 of 3 |
| 20x20 | 0 | — | — | 3 of 3 |
| 4x6x6 | 20 | 31 ms | 53 ms | — |
| 4x8x8 | 3 | 254 ms | 5.16 s | — |
| 9x6x6 | 2 | 1.06 s | 1.93 s | — |
| 8in14 | 10 | 169 ms | 41.4 s | — |
| 6in10in14 | 3 | 517 ms | 1.88 s | — |

Read the spread, not the medians. 12x12 has a median of 20 ms and a p95
of a full second — 52x. 8in14 has a median of 169 ms and a p95 of 41
seconds — 245x. The variance is not a large-grid problem that starts at
16x16; it is everywhere, and at the large sizes it simply crosses from
"slow" into "never". The 9x6x6 row reads as fast at two samples, but a
third seed took 94 s: that is the G8 miss above, and it is the same
phenomenon.

**What this blocks right now.**
- `benchmarks/baseline.json` gives every sample 150 seconds and records
  how many ran past it. A geometry that cannot be measured says so in
  the file instead of carrying an invented number, and one with an
  unfinished sample is never used by the regression guard — the guard
  would be waiting on the same unbounded carve.
- D1's sweep covers 100 puzzles up to 14x14 and for the smaller
  composites, 10 at 16x16, and **skips 18x18 and 20x20 entirely** — a
  job that can hang is worse than one that names its gap. The sweep
  prints both the reduced counts and the skipped sizes, so a green run
  never reads as full coverage.
- G8's "any special type under 30 s" stays unmet at 9x6x6.

**Decided 2026-08-29: build B4's node budget now.** The carve reports
"budget exhausted" instead of running forever, which is the only option
that keeps 18x18 and 20x20 usable rather than defining them away — and
AR26 had already said this was the prerequisite.

**Built the same day.** `SolveOutcome::BudgetExhausted` is a statement
about the search, deliberately distinct from "no solution" and from
"stuck", which are statements about the puzzle. A carve that cannot get
an answer within its budget puts the clue back — the safe direction —
and counts it, so `Carved::budget_hits` and every manifest entry record
where a puzzle paid that price.

A second unbounded path turned up while building it: after carving,
measuring the level ran a full search purely to separate "needs
guessing" from "has no solution". For a puzzle just carved out of a
solution that distinction is already settled, so the ladder alone now
decides.

Measured, one 18x18 per seed, single-threaded, before and after:

| seed | 31 | 77 | 555 | 2026 | 909 |
|---|---|---|---|---|---|
| before | 11 s | 2 s | 84 s | >80 min | not in 180 s |
| after | 10 s | 2 s | 36 s | 87 s | 53 s |

20x20, which never finished a single measurement, now takes 46 s, 159 s
and 182 s for its three baseline seeds.

**What B4 does and does not promise.** It bounds one uniqueness
question, not a whole carve, and a carve asks that question once per
cell. Termination is therefore guaranteed and the bound is loose:
measured worst case, a 16x16 that took 879 s with only two budget hits
— nearly every one of its searches was expensive but stayed under the
limit, so the budget barely engaged. "Never finishes" became "finishes,
sometimes slowly", which is the promise AR26 asked for; making it fast
as well is a separate question about the size of the budget. What the budget costs, measured against an
effectively unbounded 50 million: nothing at 12x12 (0 of 20 puzzles) and
14x14 (0 of 10); one extra clue at 8in14 (1 of 10); no extra clues at
16x16 (2 of 3). 18x18 and 20x20 are back in the baseline and in D1's
sweep.
