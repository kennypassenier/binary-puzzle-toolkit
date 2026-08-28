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
- **B4 node budget** and **B5 git revision in `--version`** remain open;
  neither blocks anything today.

## Q3 · Phase 2's three mandatory items were never discussed

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

Kenny already scheduled these for right after the L9 gate.

## Q4 · A code comment argues something that measurement contradicts

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

This is a Phase-7 "reasoned vs measured" finding; the fix is a comment,
not behaviour, so it is queued rather than applied mid-audit.
