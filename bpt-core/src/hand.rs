//! The by-hand solver: solve a puzzle using only steps a person can
//! name and check, and say which step it was.
//!
//! The main ladder (`strategy`) is built to be fast and complete, and its
//! top stage writes out every legal way to fill a line, which nobody does
//! with a pencil. This module is the opposite trade: five techniques, each
//! with a one-sentence reason, and a puzzle either finishes with them or it
//! is not solvable by hand. `bpt forge` uses it as a gate and `bpt report`
//! as the definition of "solvable by a person" (feat-human-1).
//!
//! | Technique | Reason a person gives |
//! |---|---|
//! | Pair | two equal neighbours: the cells either side must differ |
//! | Gap | equal cells with one gap: the gap must differ |
//! | FullCount | one digit already has its half of the line: the rest is the other |
//! | UniqueLine | a finished twin line: copying it would repeat a line |
//! | Capacity | a digit here and the line can no longer hold its half |

use crate::grid::{Cell, Grid};
use crate::region::Puzzle;
use crate::strategy::{
    AvoidTriple, Deduction, FillByCount, FindDuo, KeepLineUnique, LineView, Reason, Strategy,
    StrategyId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Technique {
    Pair,
    Gap,
    FullCount,
    UniqueLine,
    Capacity,
}

impl Technique {
    pub const ALL: [Technique; 5] = [
        Technique::Pair,
        Technique::Gap,
        Technique::FullCount,
        Technique::UniqueLine,
        Technique::Capacity,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Technique::Pair => "Pair",
            Technique::Gap => "Gap",
            Technique::FullCount => "FullCount",
            Technique::UniqueLine => "UniqueLine",
            Technique::Capacity => "Capacity",
        }
    }

    /// 1: seen at a glance; 2: needs a second line or a count.
    pub fn level(&self) -> u8 {
        match self {
            Technique::Pair | Technique::Gap | Technique::FullCount => 1,
            Technique::UniqueLine | Technique::Capacity => 2,
        }
    }
}

/// Why a step holds: the shared rule reasons, or a capacity certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    Rule(Reason),
    /// With `tried` here, the line can hold at most `at_most` of `digit`
    /// without three in a row, short of the `needed` it must contain;
    /// `None` means no triple-free completion exists at all.
    Capacity {
        tried: Cell,
        digit: Cell,
        needed: usize,
        at_most: Option<usize>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub row: usize,
    pub col: usize,
    pub value: Cell,
    pub technique: Technique,
    pub why: Why,
}

#[derive(Debug, Clone)]
pub struct HandResult {
    pub solved: bool,
    pub steps: Vec<Step>,
}

impl HandResult {
    pub fn count(&self, technique: Technique) -> usize {
        self.steps
            .iter()
            .filter(|s| s.technique == technique)
            .count()
    }

    /// The hardest technique level any step needed (0 for a full grid).
    pub fn level(&self) -> u8 {
        self.steps
            .iter()
            .map(|s| s.technique.level())
            .max()
            .unwrap_or(0)
    }
}

pub fn solvable_by_hand(puzzle: &Puzzle) -> bool {
    solve_by_hand(puzzle).solved
}

/// Cheap techniques first; any progress returns to the cheapest. A
/// contradiction (a deduction against a filled cell) ends the run unsolved.
pub fn solve_by_hand(puzzle: &Puzzle) -> HandResult {
    let mut grid = puzzle.givens.clone();
    let regions = puzzle.regions();
    let mut steps = Vec::new();
    let stages: [&[&dyn Strategy]; 2] =
        [&[&FindDuo, &AvoidTriple, &FillByCount], &[&KeepLineUnique]];
    'ladder: loop {
        for stage in 0..3 {
            let mut changed = false;
            for region in &regions {
                for is_row in [true, false] {
                    for index in 0..region.line_count(is_row) {
                        let found = {
                            let view = LineView::new(&grid, *region, is_row, index);
                            deductions(stage, &stages, &view)
                        };
                        match apply(&mut grid, found, &mut steps) {
                            Applied::Conflict => {
                                return HandResult {
                                    solved: false,
                                    steps,
                                };
                            }
                            Applied::Some => changed = true,
                            Applied::None => {}
                        }
                    }
                }
            }
            if changed {
                continue 'ladder;
            }
        }
        break;
    }
    HandResult {
        solved: grid.is_complete(),
        steps,
    }
}

fn deductions(stage: usize, stages: &[&[&dyn Strategy]; 2], view: &LineView<'_>) -> Vec<Step> {
    if stage == 2 {
        return capacity_steps(view);
    }
    stages[stage]
        .iter()
        .flat_map(|s| s.apply(view))
        .map(rule_step)
        .collect()
}

fn rule_step(d: Deduction) -> Step {
    let technique = match d.strategy {
        StrategyId::FindDuo => Technique::Pair,
        StrategyId::AvoidTriple => Technique::Gap,
        StrategyId::FillByCount => Technique::FullCount,
        _ => Technique::UniqueLine,
    };
    Step {
        row: d.row,
        col: d.col,
        value: d.value,
        technique,
        why: Why::Rule(d.reason),
    }
}

enum Applied {
    None,
    Some,
    Conflict,
}

fn apply(grid: &mut Grid, found: Vec<Step>, steps: &mut Vec<Step>) -> Applied {
    let mut result = Applied::None;
    for step in found {
        let current = grid.get(step.row, step.col);
        if current == step.value {
            continue;
        }
        if !current.is_empty() {
            return Applied::Conflict;
        }
        grid.set(step.row, step.col, step.value);
        steps.push(step);
        result = Applied::Some;
    }
    result
}

/// Try each digit in each open cell; a digit that leaves the line unable
/// to reach its half (or to avoid three in a row) forces the other one.
fn capacity_steps(view: &LineView<'_>) -> Vec<Step> {
    let cells = view.cells();
    let half = cells.len() / 2;
    let mut scratch = cells.to_vec();
    let mut out = Vec::new();
    for i in 0..cells.len() {
        if !cells[i].is_empty() {
            continue;
        }
        for tried in [Cell::Zero, Cell::One] {
            scratch[i] = tried;
            let shortfall = [Cell::Zero, Cell::One].into_iter().find_map(|digit| {
                let at_most = max_of(&scratch, digit);
                (at_most.is_none_or(|m| m < half)).then_some((digit, at_most))
            });
            scratch[i] = Cell::Empty;
            if let Some((digit, at_most)) = shortfall {
                let (row, col) = view.pos(i);
                out.push(Step {
                    row,
                    col,
                    value: tried.opposite().expect("tried is filled"),
                    technique: Technique::Capacity,
                    why: Why::Capacity {
                        tried,
                        digit,
                        needed: half,
                        at_most,
                    },
                });
                break;
            }
        }
    }
    out
}

/// The most copies of `digit` any triple-free completion of `cells` can
/// hold, or None when no triple-free completion exists.
fn max_of(cells: &[Cell], digit: Cell) -> Option<usize> {
    // best[last][run - 1]: most digits so far ending in `last` after `run` equal cells.
    let mut best: [[Option<usize>; 2]; 2] = [[None; 2]; 2];
    for (pos, cell) in cells.iter().enumerate() {
        let mut next: [[Option<usize>; 2]; 2] = [[None; 2]; 2];
        for (v, value) in [Cell::Zero, Cell::One].into_iter().enumerate() {
            if !cell.is_empty() && *cell != value {
                continue;
            }
            let gain = usize::from(value == digit);
            let mut push = |run: usize, from: Option<usize>| {
                if let Some(n) = from {
                    let slot = &mut next[v][run];
                    *slot = Some(slot.map_or(n + gain, |s| s.max(n + gain)));
                }
            };
            if pos == 0 {
                push(0, Some(0));
                continue;
            }
            push(1, best[v][0]);
            push(0, best[1 - v][0].max(best[1 - v][1]));
        }
        best = next;
    }
    best.iter().flatten().flatten().copied().max()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_line;

    #[test]
    fn feat_human_1_an_easy_puzzle_finishes_with_level_one_steps() {
        let p = parse_line("1..0....00.1.00..1......00.1...1..00").unwrap();
        let r = solve_by_hand(&p);
        assert!(r.solved);
        assert!(r.level() <= 2);
        assert_eq!(
            r.steps.len(),
            p.givens.cells().iter().filter(|c| c.is_empty()).count()
        );
    }

    #[test]
    fn feat_human_1_capacity_names_the_shortfall() {
        // 001..0..: zeros at 0, 1, 3, 5 and 7 is the most it can hold.
        let cells = crate::parse::parse_grid_chars("001..0..").unwrap();
        assert_eq!(max_of(&cells, Cell::Zero), Some(5));
        let none = crate::parse::parse_grid_chars("000.....").unwrap();
        assert_eq!(max_of(&none, Cell::One), None);
    }

    #[test]
    fn feat_human_1_an_empty_grid_is_not_solvable_by_hand() {
        assert!(!solvable_by_hand(&parse_line(&".".repeat(36)).unwrap()));
    }
}
