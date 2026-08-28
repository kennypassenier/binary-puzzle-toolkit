//! AR13: the core is bit-deterministic — fixed iteration order, no
//! hashing, no time dependence. Three frozen promises rest on it (K16
//! trace snapshots, trace replay, M5 parallel-equals-sequential), yet
//! nothing asserted it until the phase 7 audit pointed that out.

use bpt_core::event::{EventLog, format_trace};
use bpt_core::parse::parse_line;
use bpt_core::search::{SolveMode, solve};
use std::fs;
use std::path::{Path, PathBuf};

fn trace_of(line: &str, mode: SolveMode) -> String {
    let puzzle = parse_line(line).expect("valid puzzle");
    let mut log = EventLog::default();
    let _ = solve(&puzzle, mode, &mut log);
    format_trace(&log.events)
}

#[test]
fn ar13_same_puzzle_twice_gives_byte_identical_traces() {
    // A ladder-only puzzle and one that needs the search: the two
    // halves of the engine have different iteration paths.
    let cases = [
        "1..0....00.1.00..1......00.1...1..00",
        &".".repeat(36),
        &format!("1{}", ".".repeat(35)),
    ];
    for case in cases {
        for mode in [SolveMode::FirstSolution, SolveMode::ProveUniqueness] {
            let first = trace_of(case, mode);
            let second = trace_of(case, mode);
            assert_eq!(
                first, second,
                "{case:?} in {mode:?} produced two different traces"
            );
            assert!(!first.is_empty(), "{case:?} produced no trace at all");
        }
    }
}

#[test]
fn ar13_every_corpus_puzzle_traces_identically_across_runs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus");
    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();
    for path in &files {
        let content = fs::read_to_string(path).unwrap();
        let line = content.lines().next().unwrap();
        assert_eq!(
            trace_of(line, SolveMode::FirstSolution),
            trace_of(line, SolveMode::FirstSolution),
            "{}: trace is not reproducible",
            path.display()
        );
    }
}

/// The full trace of one fixed puzzle, pinned the way the AR7 format
/// vectors are: any reordering of steps changes this file, not just
/// step 1. Update deliberately when strategy order changes on purpose.
#[test]
fn k16_full_trace_of_a_fixed_puzzle_is_pinned() {
    let expected = include_str!("fixtures/trace/easy6.txt");
    let actual = trace_of(
        "1..0....00.1.00..1......00.1...1..00",
        SolveMode::FirstSolution,
    );
    assert_eq!(
        actual, expected,
        "the trace changed; if that was intended, update tests/fixtures/trace/easy6.txt"
    );
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("corpus readable") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "txt") {
            out.push(path);
        }
    }
}

/// B1: the solver branches through a choice oracle so a generator can
/// fill an empty grid differently each time, while the default oracle
/// keeps the solver itself bit-deterministic. Both halves of that
/// promise are asserted here.
#[test]
fn b1_a_custom_oracle_reaches_a_different_but_valid_solution() {
    use bpt_core::event::NullObserver;
    use bpt_core::grid::{Cell, Grid};
    use bpt_core::region::{Region, validate_solution};
    use bpt_core::search::{ChoiceOracle, SolveOutcome, solve_with};

    /// Mirrors the default cell choice but tries one before zero.
    struct OnesFirst;
    impl ChoiceOracle for OnesFirst {
        fn pick_cell(&mut self, grid: &Grid, regions: &[Region]) -> Option<(usize, usize)> {
            let n = grid.size();
            for r in 0..n {
                for c in 0..n {
                    if grid.get(r, c).is_empty() {
                        return Some((r, c));
                    }
                }
            }
            let _ = regions;
            None
        }
        fn value_order(&mut self, _row: usize, _col: usize) -> [Cell; 2] {
            [Cell::One, Cell::Zero]
        }
    }

    let puzzle = parse_line(&".".repeat(36)).unwrap();

    let SolveOutcome::Solved {
        solution: default_first,
        ..
    } = solve(&puzzle, SolveMode::FirstSolution, &mut NullObserver)
    else {
        panic!("an empty grid has solutions");
    };
    let SolveOutcome::Solved {
        solution: ones_first,
        ..
    } = solve_with(
        &puzzle,
        SolveMode::FirstSolution,
        &mut NullObserver,
        &mut OnesFirst,
    )
    else {
        panic!("the custom oracle must also find a solution");
    };

    assert_ne!(
        default_first, ones_first,
        "a different branching order must reach a different grid, or the oracle is not being consulted"
    );
    for grid in [&default_first, &ones_first] {
        assert!(
            validate_solution(grid, &puzzle.regions()).is_empty(),
            "every oracle must still produce a rule-valid grid"
        );
    }
    assert_eq!(
        default_first.to_line(),
        {
            let SolveOutcome::Solved { solution, .. } =
                solve(&puzzle, SolveMode::FirstSolution, &mut NullObserver)
            else {
                unreachable!()
            };
            solution.to_line()
        },
        "the default oracle stays bit-deterministic"
    );
}
