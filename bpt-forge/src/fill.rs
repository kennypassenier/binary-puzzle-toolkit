//! K20: complete an empty grid to a full valid solution. Randomness
//! lives here, never in the solver: the solver stays bit-deterministic
//! and takes a caller-supplied choice oracle (AR21).

use bpt_core::event::NullObserver;
use bpt_core::grid::{Cell, Grid};
use bpt_core::region::{Puzzle, Region};
use bpt_core::search::{ChoiceOracle, SolveMode, SolveOutcome, solve_with};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Drives the solver's branching from a seeded stream, which is what
/// turns one deterministic solver into a generator: the same grid filled
/// under different streams yields different valid solutions.
struct RandomChoice<'a> {
    rng: &'a mut ChaCha8Rng,
}

impl ChoiceOracle for RandomChoice<'_> {
    fn pick_cell(&mut self, grid: &Grid, _regions: &[Region]) -> Option<(usize, usize)> {
        // Reservoir sampling over the empty cells: one pass, no
        // allocation, and every empty cell equally likely. Picking the
        // first empty cell instead would bias every puzzle towards the
        // same shape regardless of the seed.
        let n = grid.size();
        let mut chosen = None;
        let mut seen = 0u32;
        for row in 0..n {
            for col in 0..n {
                if grid.get(row, col).is_empty() {
                    seen += 1;
                    if self.rng.random_range(0..seen) == 0 {
                        chosen = Some((row, col));
                    }
                }
            }
        }
        chosen
    }

    fn value_order(&mut self, _row: usize, _col: usize) -> [Cell; 2] {
        if self.rng.random_bool(0.5) {
            [Cell::One, Cell::Zero]
        } else {
            [Cell::Zero, Cell::One]
        }
    }
}

/// Fill `regions` on an `n`×`n` grid to a complete, valid solution.
///
/// Returns None when the geometry admits no solution at all — an
/// over-constrained invented layout, for instance. That is a property of
/// the geometry, not of the seed, so retrying with another stream will
/// not help and the caller should reject the geometry.
pub fn solution(n: usize, regions: &[Region], rng: &mut ChaCha8Rng) -> Option<Grid> {
    let puzzle = Puzzle::custom(Grid::empty(n), regions.to_vec());
    let mut oracle = RandomChoice { rng };
    match solve_with(
        &puzzle,
        SolveMode::FirstSolution,
        &mut NullObserver,
        &mut oracle,
    ) {
        SolveOutcome::Solved { solution, .. } => Some(solution),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::stream;
    use bpt_core::region::validate_solution;

    fn standard(n: usize) -> Vec<Region> {
        vec![Region::square(0, 0, n)]
    }

    #[test]
    fn k20_fills_an_empty_grid_to_a_valid_solution() {
        for n in [6, 8, 10] {
            let regions = standard(n);
            let grid = solution(n, &regions, &mut stream(1, n as u64, 0))
                .unwrap_or_else(|| panic!("{n}x{n} must be fillable"));
            assert!(grid.is_complete(), "{n}x{n}: grid must be full");
            assert!(
                validate_solution(&grid, &regions).is_empty(),
                "{n}x{n}: filled grid must satisfy every rule"
            );
        }
    }

    #[test]
    fn m20_the_same_seed_gives_the_same_solution() {
        let regions = standard(8);
        let a = solution(8, &regions, &mut stream(99, 0, 0)).unwrap();
        let b = solution(8, &regions, &mut stream(99, 0, 0)).unwrap();
        assert_eq!(a, b, "generation must be reproducible from its seed");
    }

    #[test]
    fn k20_different_seeds_give_different_solutions() {
        let regions = standard(8);
        let mut seen = std::collections::HashSet::new();
        for index in 0..8 {
            let grid = solution(8, &regions, &mut stream(7, index, 0)).unwrap();
            seen.insert(grid.to_line());
        }
        assert!(
            seen.len() >= 7,
            "eight seeds produced only {} distinct grids — the stream is barely reaching the search",
            seen.len()
        );
    }

    #[test]
    fn k22_fills_a_composite_geometry() {
        // 4x6x6: four blocks plus the whole grid, all constrained at once.
        let regions = vec![
            Region::square(0, 0, 6),
            Region::square(0, 6, 6),
            Region::square(6, 0, 6),
            Region::square(6, 6, 6),
            Region::square(0, 0, 12),
        ];
        let grid = solution(12, &regions, &mut stream(5, 0, 0)).expect("4x6x6 must be fillable");
        assert!(
            validate_solution(&grid, &regions).is_empty(),
            "every block and the whole grid must hold"
        );
    }
}
