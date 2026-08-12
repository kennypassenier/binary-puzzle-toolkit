//! The solve engine: strategy ladder to fixpoint, then DFS with
//! cheap-tier propagation (AR5), producing the four AR6 outcomes.

use crate::event::{Observer, SolveEvent};
use crate::grid::{Cell, Grid};
use crate::region::{Puzzle, Region, Violation, validate_partial, validate_solution};
use crate::strategy::{Deduction, LineView, Strategy, registry_stages};
use std::fmt;

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

/// How far the solver may go (AR5/AR6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveMode {
    /// M1: human strategies only, never guess.
    StrategiesOnly,
    /// K4: stop at the first solution.
    FirstSolution,
    /// K5: search past the first solution to prove it unique.
    ProveUniqueness,
}

/// Work done up to the FIRST solution (AR8: uniqueness refutation work
/// is excluded so M2 grading measures the puzzle, not the proof).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SolveStats {
    pub deductions: usize,
    pub guesses: usize,
    pub backtracks: usize,
    /// Highest strategy tier that produced a deduction (M2 input).
    pub max_tier: u8,
}

/// M2 difficulty grade: the hardest reasoning the solver actually
/// needed. Calibrated against binarypuzzle.com's own labels over the
/// corpus (tests/difficulty.rs prints the table).
///
/// Three bands, not four: the site's "easy" and "medium" puzzles both
/// fall to tier 1–2 strategies alone, so no measurement here separates
/// them — `Easy` covers both. Site "hard" needs tier 3, site "very
/// hard" needs tier 4 or guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Difficulty {
    /// Local patterns and line counts suffice (site: easy or medium).
    Easy,
    /// Cross-line reasoning needed (site: hard).
    Hard,
    /// Line enumeration or guessing needed (site: very hard).
    VeryHard,
}

impl Difficulty {
    pub fn name(&self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Hard => "hard",
            Difficulty::VeryHard => "very hard",
        }
    }
}

/// Grade a solved puzzle from its stats (M2).
pub fn grade(stats: &SolveStats) -> Difficulty {
    if stats.guesses > 0 {
        return Difficulty::VeryHard;
    }
    match stats.max_tier {
        0..=2 => Difficulty::Easy,
        3 => Difficulty::Hard,
        _ => Difficulty::VeryHard,
    }
}

/// Why a puzzle has no solution (K6): always concrete, never generic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContradictionReason {
    /// The givens already break a rule.
    Invalid(Violation),
    /// Two sound deductions demand different values for one cell.
    ConflictingDeductions { row: usize, col: usize },
    /// Every assignment of the open cells breaks a rule somewhere.
    Exhausted,
}

impl fmt::Display for ContradictionReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContradictionReason::Invalid(v) => write!(f, "{v}"),
            ContradictionReason::ConflictingDeductions { row, col } => write!(
                f,
                "cell r{row}c{col} is forced to both 0 and 1 by different rules"
            ),
            ContradictionReason::Exhausted => write!(
                f,
                "no assignment of the open cells satisfies all rules (search exhausted)"
            ),
        }
    }
}

/// The AR6 outcome. `Stuck` only occurs in StrategiesOnly mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveOutcome {
    Solved { solution: Grid, stats: SolveStats },
    MultipleSolutions { first: Grid, second: Grid },
    Contradiction { reason: ContradictionReason },
    Stuck { grid: Grid, filled: usize },
}

/// Solve a puzzle (K4/K5/M1). Ladder first; if needed and allowed, DFS
/// with cheap-tier propagation. Deterministic throughout (AR13).
pub fn solve(puzzle: &Puzzle, mode: SolveMode, observer: &mut dyn Observer) -> SolveOutcome {
    let regions = puzzle.regions();
    let invalid = validate_partial(&puzzle.givens, &regions);
    if let Some(v) = invalid.into_iter().next() {
        return SolveOutcome::Contradiction {
            reason: ContradictionReason::Invalid(v),
        };
    }
    let mut stats = SolveStats::default();
    let mut counting = CountingObserver {
        inner: observer,
        deductions: 0,
        max_tier: 0,
        counting: true,
    };
    let grid = match run_to_fixpoint(puzzle, &mut counting) {
        StrategyRun::Contradiction { row, col } => {
            return SolveOutcome::Contradiction {
                reason: ContradictionReason::ConflictingDeductions { row, col },
            };
        }
        StrategyRun::Solved(grid) => {
            // "No empty cells" is not the same as "valid": strategies
            // fill cells from local rules and can complete a grid that
            // breaks a global rule (found by the M7 fuzzer on ".10.").
            if let Some(v) = validate_solution(&grid, &regions).into_iter().next() {
                return SolveOutcome::Contradiction {
                    reason: ContradictionReason::Invalid(v),
                };
            }
            // A valid ladder solution needs no uniqueness search: the
            // ladder only ever makes forced deductions.
            stats.deductions = counting.deductions;
            stats.max_tier = counting.max_tier;
            counting.inner.on_event(&SolveEvent::SolutionFound);
            return SolveOutcome::Solved {
                solution: grid,
                stats,
            };
        }
        StrategyRun::Stuck { grid, filled } => {
            if mode == SolveMode::StrategiesOnly {
                return SolveOutcome::Stuck { grid, filled };
            }
            grid
        }
    };
    stats.deductions = counting.deductions;
    stats.max_tier = counting.max_tier;
    let want = match mode {
        SolveMode::FirstSolution => 1,
        _ => 2,
    };
    let mut ctx = SearchCtx {
        regions: &regions,
        stages: registry_stages(),
        observer: counting.inner,
        stats: &mut stats,
        solutions: Vec::new(),
        want,
    };
    dfs(grid, 0, &mut ctx);
    let mut solutions = ctx.solutions;
    match solutions.len() {
        0 => SolveOutcome::Contradiction {
            reason: ContradictionReason::Exhausted,
        },
        1 => SolveOutcome::Solved {
            solution: solutions.pop().expect("one solution"),
            stats,
        },
        _ => {
            let second = solutions.pop().expect("two solutions");
            let first = solutions.pop().expect("two solutions");
            SolveOutcome::MultipleSolutions { first, second }
        }
    }
}

struct CountingObserver<'a> {
    inner: &'a mut dyn Observer,
    deductions: usize,
    max_tier: u8,
    counting: bool,
}

impl Observer for CountingObserver<'_> {
    fn on_event(&mut self, event: &SolveEvent) {
        if self.counting
            && let SolveEvent::Deduced { strategy, .. } = event
        {
            self.deductions += 1;
            self.max_tier = self.max_tier.max(strategy.tier());
        }
        self.inner.on_event(event);
    }
}

struct SearchCtx<'a> {
    regions: &'a [Region],
    stages: Vec<Vec<Box<dyn Strategy>>>,
    observer: &'a mut dyn Observer,
    stats: &'a mut SolveStats,
    solutions: Vec<Grid>,
    want: usize,
}

impl SearchCtx<'_> {
    fn counting(&self) -> bool {
        self.solutions.is_empty()
    }
}

/// Depth-first search with cheap-tier propagation at every node (AR5).
/// Returns true when enough solutions were found to stop.
fn dfs(mut grid: Grid, depth: usize, ctx: &mut SearchCtx<'_>) -> bool {
    // Propagate with the cheap stage only (tiers 1-2); deductions still
    // reach the observer so traces show the search's reasoning.
    let before = grid.filled_count();
    loop {
        let SearchCtx {
            regions,
            stages,
            observer,
            ..
        } = ctx;
        match single_pass(&mut grid, regions, &stages[0], *observer) {
            PassResult::Contradiction { .. } => return false,
            PassResult::Changed => continue,
            PassResult::Fixpoint => break,
        }
    }
    if ctx.counting() {
        ctx.stats.deductions += grid.filled_count() - before;
    }
    if grid.is_complete() {
        if !validate_solution(&grid, ctx.regions).is_empty() {
            return false;
        }
        if ctx.solutions.iter().all(|s| *s != grid) {
            ctx.observer.on_event(&SolveEvent::SolutionFound);
            ctx.solutions.push(grid);
        }
        return ctx.solutions.len() >= ctx.want;
    }
    let Some((row, col)) = pick_guess_cell(&grid, ctx.regions) else {
        return false;
    };
    for value in [Cell::Zero, Cell::One] {
        let mut branch = grid.clone();
        branch.set(row, col, value);
        if ctx.counting() {
            ctx.stats.guesses += 1;
        }
        ctx.observer.on_event(&SolveEvent::Guessed {
            row,
            col,
            value,
            depth,
        });
        if dfs(branch, depth + 1, ctx) {
            return true;
        }
        if ctx.counting() {
            ctx.stats.backtracks += 1;
        }
        ctx.observer.on_event(&SolveEvent::Backtracked {
            to_depth: depth,
            row,
            col,
        });
    }
    false
}

/// AR5 heuristic: the line (any region, rows before columns) with the
/// fewest empty cells (>0); first such line on ties; guess its first
/// empty cell.
fn pick_guess_cell(grid: &Grid, regions: &[Region]) -> Option<(usize, usize)> {
    let mut best: Option<(usize, (usize, usize))> = None;
    for region in regions {
        for is_row in [true, false] {
            for index in 0..region.n {
                let view = LineView::new(grid, *region, is_row, index);
                let empties: Vec<usize> = (0..view.len())
                    .filter(|i| view.cells()[*i].is_empty())
                    .collect();
                if empties.is_empty() {
                    continue;
                }
                let cell = view.pos(empties[0]);
                if best.is_none_or(|(count, _)| empties.len() < count) {
                    best = Some((empties.len(), cell));
                }
            }
        }
    }
    best.map(|(_, cell)| cell)
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
