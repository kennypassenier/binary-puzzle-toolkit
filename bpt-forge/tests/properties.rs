//! M28: property tests over the whole pipeline.
//!
//! Unit tests check the cases someone thought of. These check the four
//! things that must hold for *every* geometry, size and seed, and when
//! one fails proptest shrinks it to a minimal case that M20 can replay
//! from its `(seed, index, attempt)`.
//!
//! Sizes stay small — 4 to 10 — because the properties are about
//! correctness, not scale, and a property test that takes a minute per
//! case gets run with too few cases to find anything.

use bpt_core::event::NullObserver;
use bpt_core::grid::Cell;
use bpt_core::region::{Puzzle, Region};
use bpt_core::search::{SolveMode, SolveOutcome, solve};
use bpt_forge::batch::{self, Plan};
use bpt_forge::carve::carve;
use bpt_forge::fill;
use bpt_forge::grade::{Level, level};
use bpt_forge::rng;
use proptest::prelude::*;

fn level_of(index: usize) -> Level {
    [Level::L1, Level::L2, Level::L3, Level::L4][index]
}

/// A geometry built from the parameters: a plain grid, or a grid with a
/// nested inner region. Both shapes the generator has to handle, without
/// hard-coding a list of known types.
fn geometry(n: usize, inset: usize) -> Vec<Region> {
    let mut regions = vec![Region::square(0, 0, n)];
    // An inner region must be even-sided and at least 4x4 to be a binary
    // puzzle in its own right.
    let inner = n - 2 * inset;
    if inset > 0 && inner >= 4 {
        regions.push(Region::square(inset, inset, inner));
    }
    regions
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    /// Every emitted puzzle has exactly one solution. This is the claim
    /// the whole generator exists to make.
    #[test]
    fn m28_every_generated_puzzle_is_uniquely_solvable(
        size_index in 0usize..4,
        inset in 0usize..3,
        seed: u64,
        level_index in 0usize..4,
    ) {
        let n = [4, 6, 8, 10][size_index];
        let regions = geometry(n, inset);
        let mut r = rng::stream(seed, 0, 0);
        let Some(solution) = fill::solution(n, &regions, &mut r) else {
            // An over-constrained geometry has no solutions at all, which
            // is a legitimate answer, not a failure.
            return Ok(());
        };
        let carved = carve(&solution, &regions, level_of(level_index), &mut r);
        let puzzle = Puzzle::custom(carved.puzzle.clone(), regions.clone());
        prop_assert!(
            matches!(
                solve(&puzzle, SolveMode::ProveUniqueness, &mut NullObserver),
                SolveOutcome::Solved { .. }
            ),
            "n={n} inset={inset} seed={seed}: the emitted puzzle is not uniquely solvable"
        );
    }

    /// A puzzle is its solution with cells taken out — never a cell
    /// changed. A single flipped clue would make the puzzle unsolvable
    /// or, worse, solvable to something else.
    #[test]
    fn m28_a_puzzle_is_a_subset_of_its_solution(
        size_index in 0usize..4,
        inset in 0usize..3,
        seed: u64,
    ) {
        let n = [4, 6, 8, 10][size_index];
        let regions = geometry(n, inset);
        let mut r = rng::stream(seed, 0, 0);
        let Some(solution) = fill::solution(n, &regions, &mut r) else {
            return Ok(());
        };
        let carved = carve(&solution, &regions, Level::L4, &mut r);
        for row in 0..n {
            for col in 0..n {
                let clue = carved.puzzle.get(row, col);
                if clue != Cell::Empty {
                    prop_assert_eq!(
                        clue,
                        carved.solution.get(row, col),
                        "n={} seed={} cell ({},{}) disagrees with the solution", n, seed, row, col
                    );
                }
            }
        }
        prop_assert_eq!(carved.solution, solution);
    }

    /// Grading is a property of the puzzle, not of the run that made it:
    /// measuring the same puzzle twice gives the same level, and it is
    /// the level the batch recorded.
    #[test]
    fn m28_the_measured_level_is_reproducible(
        size_index in 0usize..4,
        inset in 0usize..3,
        seed: u64,
        level_index in 0usize..4,
    ) {
        let n = [4, 6, 8, 10][size_index];
        let regions = geometry(n, inset);
        let ceiling = level_of(level_index);
        let mut r = rng::stream(seed, 0, 0);
        let Some(solution) = fill::solution(n, &regions, &mut r) else {
            return Ok(());
        };
        let carved = carve(&solution, &regions, ceiling, &mut r);
        let puzzle = Puzzle::custom(carved.puzzle.clone(), regions.clone());
        let again = level(&puzzle).expect("a carved puzzle is solvable");
        prop_assert_eq!(carved.level, again);
        prop_assert!(carved.level <= ceiling, "AR23: the ceiling is an invariant");
    }

    /// The three numbers a manifest stores really do rebuild the puzzle,
    /// for any geometry — not only for the ones with a fixture.
    #[test]
    fn m28_a_batch_regenerates_from_its_recorded_triples(
        size_index in 0usize..3,
        inset in 0usize..2,
        seed: u64,
    ) {
        let n = [4, 6, 8][size_index];
        let regions = geometry(n, inset);
        let plan = Plan::new(n, regions, Level::L4, seed, 3);
        let outcome = batch::run(&plan, &mut std::collections::HashSet::new());
        for produced in &outcome.produced {
            let again = batch::regenerate(&plan, produced.index, produced.attempt)
                .expect("what was generated once regenerates");
            prop_assert_eq!(again.puzzle.to_line(), produced.carved.puzzle.to_line());
        }
    }
}
