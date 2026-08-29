//! K24/K25 with the AR24 amendment: the difficulty of a puzzle is the
//! lowest ladder stage that cracks it. Level membership belongs to the
//! solver, so this module only reads what a solve reports rather than
//! defining techniques of its own.

use bpt_core::event::NullObserver;
use bpt_core::region::Puzzle;
use bpt_core::search::{SolveMode, SolveOutcome, solve};

use serde::{Deserialize, Serialize};

/// The generator's four-level scale (AR24).
///
/// Deliberately different from the solver's own three-band grade, which
/// merges the site's easy and medium because nothing separates them on
/// real puzzles. Here the levels are stages of the ladder, which are
/// directly measurable, and telling L1 from L2 is the whole point of
/// being able to *target* a difficulty.
/// Serialized as its own name (`"L3"`), so a manifest reads the way the
/// `--level` flag is written rather than as a struct variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Level {
    /// Local patterns and line counts suffice (ladder stage 0).
    L1,
    /// Cross-line reasoning needed (stage 1).
    L2,
    /// Line enumeration needed (stage 2).
    L3,
    /// No amount of forced reasoning finishes it; guessing required.
    L4,
}

impl Level {
    pub fn name(&self) -> &'static str {
        match self {
            Level::L1 => "L1",
            Level::L2 => "L2",
            Level::L3 => "L3",
            Level::L4 => "L4",
        }
    }
}

/// Measure the level of a puzzle already known to be solvable.
///
/// `level` runs a full search to separate "needs guessing" from "has no
/// solution", and on a large grid that search is exactly the unbounded
/// one B4 exists to bound. A caller that carved the puzzle out of a
/// solution does not need the distinction — the solution still solves
/// it — so the ladder alone decides, and a stall means L4.
pub fn level_of_solvable(puzzle: &Puzzle) -> Level {
    match solve(puzzle, SolveMode::StrategiesOnly, &mut NullObserver) {
        SolveOutcome::Solved { stats, .. } => match stats.max_tier {
            0..=2 => Level::L1,
            3 => Level::L2,
            _ => Level::L3,
        },
        _ => Level::L4,
    }
}

/// Measure a puzzle's level, or None when it has no solution at all.
///
/// Two solves at most: the strategies-only run answers L1..L3 directly,
/// and only a puzzle that stalls needs the full search to distinguish
/// "needs guessing" from "impossible".
pub fn level(puzzle: &Puzzle) -> Option<Level> {
    match solve(puzzle, SolveMode::StrategiesOnly, &mut NullObserver) {
        SolveOutcome::Solved { stats, .. } => Some(match stats.max_tier {
            0..=2 => Level::L1,
            3 => Level::L2,
            _ => Level::L3,
        }),
        SolveOutcome::Stuck { .. } => {
            match solve(puzzle, SolveMode::FirstSolution, &mut NullObserver) {
                SolveOutcome::Solved { .. } => Some(Level::L4),
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bpt_core::parse::parse_line;

    #[test]
    fn k24_a_solved_grid_is_the_easiest_level() {
        // Nothing left to deduce: no stage is needed at all.
        let full = parse_line("0011110001011010").unwrap();
        assert_eq!(level(&full), Some(Level::L1));
    }

    #[test]
    fn k24_real_puzzles_land_on_the_scale() {
        let content = include_str!("../../corpus/standard/6/bp-s6l1n1-20260812-easy.txt");
        let puzzle = parse_line(content.lines().next().unwrap()).unwrap();
        let measured = level(&puzzle).expect("a published puzzle has a level");
        assert!(
            measured <= Level::L3,
            "a published easy puzzle must not need guessing, got {}",
            measured.name()
        );
    }

    #[test]
    fn k24_an_impossible_puzzle_has_no_level() {
        let contradictory = parse_line(&format!("000{}", ".".repeat(33))).unwrap();
        assert_eq!(level(&contradictory), None);
    }

    #[test]
    fn k24_a_nearly_empty_grid_needs_guessing() {
        let sparse = parse_line(&".".repeat(36)).unwrap();
        assert_eq!(level(&sparse), Some(Level::L4));
    }
}
