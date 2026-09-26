<!-- id-translations: enforced -->

# Identifier translations — binary-puzzle-toolkit

One row per identifier that has moved from the old shape to the house
scheme (standing rule 4, policy set 2026-09-09). Nothing is renamed in
bulk: an old identifier is translated only when it surfaces by itself.
docs/ID_MAP.md is a different record: how the two halves' IDs were
renumbered at the 2026-08-28 merge.

The old name may not survive anywhere else in the tracked files once it
is listed here; `.githooks/check-ids.sh` refuses the commit otherwise.
This file is the one place the old names live.

| Old | New | Moved | Why it surfaced |
| --- | --- | ----- | --------------- |
| M6 | feat-json-1 | 2026-09-26 | Re-rated in the decision round after the Windows run (Later → Desired). `bpt-forge/tests/inspect_snapshots.rs` also said M6, left over from the generator's pre-merge numbering; that one meant the inspector and now says M25 |
| M27 | feat-sheets-1 | 2026-09-26 | Re-rated in the same round; stays Later |
| M29 | feat-report-1 | 2026-09-26 | Re-rated in the same round (Later → Essential) after kp-soft.dev's puzzles proved hard to solve by hand |
