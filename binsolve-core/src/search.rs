//! The solve engine. L2 scope: strategy passes to fixpoint (AR5's top
//! level). The DFS stage and the full AR6 outcome type land in L4.

use crate::event::{Observer, SolveEvent};
use crate::grid::Grid;
use crate::region::{Puzzle, Region};
use crate::strategy::{Deduction, LineView, Strategy, registry_stages};

/// Result of running strategies alone (M1's mode; L4 wraps this).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyRun {
    Solved(Grid),
    Stuck {
        grid: Grid,
        filled: usize,
    },
    /// A deduction demanded the opposite of an already-filled cell: the
    /// givens are contradictory. Full AR6 reporting lands in L4.
    Contradiction {
        row: usize,
        col: usize,
    },
}

/// Apply the strategy stages over every line of every region until no
/// deduction fires (fixpoint). A cheaper stage is exhausted before a
/// costlier one runs, and any progress drops back to the cheapest stage
/// (Tatham's ladder; AR5). Iteration order is fixed: regions in
/// decomposition order, rows before columns, strategies in tier order
/// (AR13 determinism).
pub fn run_to_fixpoint(puzzle: &Puzzle, observer: &mut dyn Observer) -> StrategyRun {
    let mut grid = puzzle.givens.clone();
    let regions = puzzle.regions();
    let stages = registry_stages();
    'ladder: loop {
        for stage in &stages {
            match single_pass(&mut grid, &regions, stage, observer) {
                PassResult::Contradiction { row, col } => {
                    return StrategyRun::Contradiction { row, col };
                }
                PassResult::Changed => continue 'ladder,
                PassResult::Fixpoint => {}
            }
        }
        break;
    }
    if grid.is_complete() {
        StrategyRun::Solved(grid)
    } else {
        let filled = grid.filled_count();
        StrategyRun::Stuck { grid, filled }
    }
}

enum PassResult {
    Changed,
    Fixpoint,
    Contradiction { row: usize, col: usize },
}

fn single_pass(
    grid: &mut Grid,
    regions: &[Region],
    strategies: &[Box<dyn Strategy>],
    observer: &mut dyn Observer,
) -> PassResult {
    let mut changed = false;
    for region in regions {
        for is_row in [true, false] {
            for index in 0..region.n {
                for strategy in strategies {
                    let deductions = {
                        let view = LineView::new(grid, *region, is_row, index);
                        strategy.apply(&view)
                    };
                    match apply_deductions(grid, &deductions, observer) {
                        Applied::Contradiction { row, col } => {
                            return PassResult::Contradiction { row, col };
                        }
                        Applied::Some => changed = true,
                        Applied::None => {}
                    }
                }
            }
        }
    }
    if changed {
        PassResult::Changed
    } else {
        PassResult::Fixpoint
    }
}

enum Applied {
    None,
    Some,
    Contradiction { row: usize, col: usize },
}

fn apply_deductions(
    grid: &mut Grid,
    deductions: &[Deduction],
    observer: &mut dyn Observer,
) -> Applied {
    let mut any = false;
    for d in deductions {
        let current = grid.get(d.row, d.col);
        if current == d.value {
            continue;
        }
        if !current.is_empty() {
            return Applied::Contradiction {
                row: d.row,
                col: d.col,
            };
        }
        grid.set(d.row, d.col, d.value);
        observer.on_event(&SolveEvent::Deduced {
            row: d.row,
            col: d.col,
            value: d.value,
            strategy: d.strategy,
            reason: d.reason,
        });
        any = true;
    }
    if any { Applied::Some } else { Applied::None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventLog, NullObserver, format_trace};
    use crate::parse::parse_line;

    #[test]
    fn k3_supplement_line_readme_scenario_via_engine() {
        // README: 0.0.0. -> 010101 (gap fills, then count completes).
        let mut line = String::from("0.0.0.");
        line.push_str(&".".repeat(30));
        let puzzle = parse_line(&line).unwrap();
        let result = run_to_fixpoint(&puzzle, &mut NullObserver);
        let StrategyRun::Stuck { grid, .. } = result else {
            panic!("6x6 with one row given cannot be fully solved: {result:?}");
        };
        let row0: String = (0..6).map(|c| grid.get(0, c).to_char()).collect();
        assert_eq!(row0, "010101");
    }

    #[test]
    fn k16_trace_of_supplement_line_is_stable() {
        let mut line = String::from("0.0.0.");
        line.push_str(&".".repeat(30));
        let puzzle = parse_line(&line).unwrap();
        let mut log = EventLog::default();
        let _ = run_to_fixpoint(&puzzle, &mut log);
        let trace = format_trace(&log.events);
        // AR13: this trace is bit-deterministic across runs/platforms.
        assert!(trace.starts_with(
            "step 1: AvoidTriple — r0c1 = 1 (cells r0c0 and r0c2 are both 0, the cell between must differ)\n"
        ));
        assert!(trace.contains("FillByCount"));
    }

    #[test]
    fn k6_contradictory_deduction_detected() {
        // "00.11.": the 00 pair forces c2=1 while the 11 pair forces
        // c2=0 — irreconcilable, and the engine must say so rather than
        // silently pick a winner.
        let mut line = String::from("00.11.");
        line.push_str(&".".repeat(30));
        let puzzle = parse_line(&line).unwrap();
        let result = run_to_fixpoint(&puzzle, &mut NullObserver);
        assert!(
            matches!(result, StrategyRun::Contradiction { row: 0, col: 2 }),
            "{result:?}"
        );
    }

    #[test]
    fn k3_easy_corpus_puzzles_solve_strategy_only() {
        let easy = [
            include_str!("../../corpus/standard/6/bp-s6l1n1-20260812-easy.txt"),
            include_str!("../../corpus/standard/8/bp-s8l1n1-20260812-easy.txt"),
            include_str!("../../corpus/standard/14/bp-s14l1n1-20260812-easy.txt"),
        ];
        for content in easy {
            let mut lines = content.lines();
            let puzzle = parse_line(lines.next().unwrap()).unwrap();
            let expected = lines.next().unwrap().strip_prefix("solution:").unwrap();
            match run_to_fixpoint(&puzzle, &mut NullObserver) {
                StrategyRun::Solved(grid) => assert_eq!(grid.to_line(), expected),
                other => panic!(
                    "easy puzzle should solve with tier 1-2 strategies, got {}",
                    match other {
                        StrategyRun::Stuck { filled, grid } =>
                            format!("stuck at {filled}/{} cells", grid.size() * grid.size()),
                        _ => format!("{other:?}"),
                    }
                ),
            }
        }
    }
}
