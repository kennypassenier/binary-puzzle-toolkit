//! K21/K25: remove clues from a full solution, proving uniqueness after
//! each removal, and restore-and-lock any removal that pushes the puzzle
//! past the requested difficulty (AR23).
//!
//! Why restore-and-lock rather than carve-to-minimal-then-repair: on
//! measured data, 23 of 28 minimal puzzles were unsolvable by any tier
//! without guessing — outside the scale entirely — and restoring clues
//! does not monotonically lower difficulty, so a repair loop can thrash
//! and then report a level unreachable that in fact is not. Keeping a
//! ceiling converges in one pass and never restarts.

use crate::grade::{Level, level};
use bpt_core::event::NullObserver;
use bpt_core::grid::{Cell, Grid};
use bpt_core::region::{Puzzle, Region};
use bpt_core::search::{SolveMode, SolveOutcome, solve};
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

/// What a carve produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carved {
    pub puzzle: Grid,
    pub solution: Grid,
    pub level: Level,
    /// Cells still filled in the puzzle.
    pub clues: usize,
}

/// Carve `solution` down to a puzzle that is uniquely solvable and no
/// harder than `ceiling`.
///
/// Every intermediate state is proven unique before it is accepted, so
/// the result carries the same guarantee as a published puzzle: exactly
/// one solution, verifiable without an answer key.
pub fn carve(solution: &Grid, regions: &[Region], ceiling: Level, rng: &mut ChaCha8Rng) -> Carved {
    let n = solution.size();
    let mut working = solution.clone();

    let mut order: Vec<(usize, usize)> = (0..n).flat_map(|r| (0..n).map(move |c| (r, c))).collect();
    order.shuffle(rng);

    for (row, col) in order {
        let removed = working.get(row, col);
        if removed.is_empty() {
            continue;
        }
        working.set(row, col, Cell::Empty);

        // At L4 the ceiling cannot reject anything — every candidate is
        // solvable by construction and L4 is the top of the scale — so
        // the ladder is not run at all in the default case. Below L4 it
        // decides, and it is the only thing that has to.
        let acceptable = still_unique_without(&working, regions, row, col, removed)
            && (ceiling == Level::L4
                || fits_ceiling(&Puzzle::custom(working.clone(), regions.to_vec()), ceiling));
        // A removal that costs uniqueness, or that pushes the puzzle past
        // the requested ceiling, is put back and never tried again.
        if !acceptable {
            working.set(row, col, removed);
        }
    }

    let puzzle = Puzzle::custom(working.clone(), regions.to_vec());
    let measured = level(&puzzle).expect("a carved puzzle is solvable by construction");
    Carved {
        clues: working.filled_count(),
        puzzle: working,
        solution: solution.clone(),
        level: measured,
    }
}

/// Is this candidate at most as hard as `ceiling`?
///
/// `level` answers a harder question than the loop asks. Its expensive
/// half is the fallback that separates "needs guessing" from "has no
/// solution at all" by running a full search — and inside carve that
/// distinction is never in doubt, because every candidate is solvable by
/// construction: the solution it was carved from still solves it.
///
/// So the ladder alone decides. It stalls exactly on the puzzles that
/// need guessing, which are L4, and those pass only when L4 is the
/// ceiling. Measured on a 16x16: with the search still in the loop one
/// carve took 11.7 s, without it 5.5 s, and the puzzles produced are
/// byte for byte the same — which the restore drill re-checks against a
/// batch generated before either change.
fn fits_ceiling(candidate: &Puzzle, ceiling: Level) -> bool {
    match solve(candidate, SolveMode::StrategiesOnly, &mut NullObserver) {
        SolveOutcome::Solved { stats, .. } => {
            let reached = match stats.max_tier {
                0..=2 => Level::L1,
                3 => Level::L2,
                _ => Level::L3,
            };
            reached <= ceiling
        }
        // Stalled: solvable but not without guessing, which is L4.
        SolveOutcome::Stuck { .. } => ceiling == Level::L4,
        // Cannot happen while the invariant holds; refusing is the safe
        // answer if it ever stops holding.
        _ => false,
    }
}

/// Does the puzzle still have exactly one solution now that (row, col)
/// is empty?
///
/// The direct reading — "prove the whole puzzle unique again" — is what
/// this used to do, and it is what made carving unusable above 14x14:
/// one 16x16 took 30.8 s against 130 ms at 14x14, because proving
/// uniqueness means exhausting a search tree that grows with every clue
/// removed.
///
/// The cheap reading is exactly equivalent. Before the removal the
/// puzzle had exactly one solution, in which this cell held `was`. Every
/// solution of the emptied puzzle therefore either holds `was` here — of
/// which there is precisely one, the original — or holds the opposite.
/// So uniqueness survives if and only if pinning the opposite value
/// yields no solution at all, and refuting a wrong value is fast: it
/// contradicts something nearby instead of exploring the whole space.
///
/// The invariant this rests on is carve's own: it starts from a complete
/// solution and never accepts a removal that breaks uniqueness, so the
/// pre-removal puzzle is unique at every step.
fn still_unique_without(
    working: &Grid,
    regions: &[Region],
    row: usize,
    col: usize,
    was: Cell,
) -> bool {
    let opposite = match was {
        Cell::Zero => Cell::One,
        Cell::One => Cell::Zero,
        // Only a filled cell is ever removed, so this cannot arise; if it
        // ever did, refusing the removal is the safe answer.
        Cell::Empty => return false,
    };
    let mut probe = working.clone();
    probe.set(row, col, opposite);
    !matches!(
        solve(
            &Puzzle::custom(probe, regions.to_vec()),
            SolveMode::FirstSolution,
            &mut NullObserver,
        ),
        SolveOutcome::Solved { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fill;
    use crate::rng::stream;
    use bpt_core::region::validate_givens;

    fn standard(n: usize) -> Vec<Region> {
        vec![Region::square(0, 0, n)]
    }

    fn carved_6x6(seed: u64, ceiling: Level) -> Carved {
        let regions = standard(6);
        let mut rng = stream(seed, 0, 0);
        let solution = fill::solution(6, &regions, &mut rng).expect("6x6 fills");
        carve(&solution, &regions, ceiling, &mut rng)
    }

    #[test]
    fn k21_a_carved_puzzle_is_uniquely_solvable() {
        let out = carved_6x6(1, Level::L4);
        let puzzle = Puzzle::custom(out.puzzle.clone(), standard(6));
        let outcome = solve(&puzzle, SolveMode::ProveUniqueness, &mut NullObserver);
        let SolveOutcome::Solved { solution, .. } = outcome else {
            panic!("a carved puzzle must have exactly one solution: {outcome:?}");
        };
        assert_eq!(
            solution, out.solution,
            "the unique solution must be the one it was carved from"
        );
    }

    #[test]
    fn k21_carving_removes_clues_but_keeps_the_givens_consistent() {
        let out = carved_6x6(2, Level::L4);
        assert!(
            out.clues < 36,
            "carving must remove something, kept all {} cells",
            out.clues
        );
        assert!(out.clues > 0, "carving must not empty the grid");
        assert!(
            validate_givens(&out.puzzle, &out.solution).is_empty(),
            "every remaining clue must match the solution it came from"
        );
    }

    #[test]
    fn k25_the_ceiling_is_respected() {
        // Asking for L1 must never hand back something that needs more.
        for seed in 1..4 {
            let out = carved_6x6(seed, Level::L1);
            assert!(
                out.level <= Level::L1,
                "seed {seed}: asked for at most L1, got {}",
                out.level.name()
            );
        }
    }

    #[test]
    fn k25_a_higher_ceiling_carves_further() {
        // Stated over seeds rather than per seed on purpose. Carving is
        // greedy, so it is not monotone in the ceiling: a removal that a
        // higher ceiling accepts changes the grid, and can block a later
        // removal the lower ceiling would have taken. A single seed can
        // therefore come out one clue heavier at L4 than at L1 without
        // anything being wrong. The direction only holds in aggregate.
        let seeds = 1..21;
        let (mut easy, mut free) = (0usize, 0usize);
        for seed in seeds.clone() {
            easy += carved_6x6(seed, Level::L1).clues;
            free += carved_6x6(seed, Level::L4).clues;
        }
        assert!(
            free < easy,
            "over {} seeds L4 kept {free} clues and L1 kept {easy} — \
             a higher ceiling must carve deeper on average",
            seeds.count()
        );
    }

    #[test]
    fn m20_carving_is_reproducible_from_its_seed() {
        assert_eq!(carved_6x6(42, Level::L4), carved_6x6(42, Level::L4));
    }
}
