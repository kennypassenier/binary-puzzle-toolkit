//! L4 integration tests: completeness (K4), uniqueness proof (K5),
//! contradiction reasons (K6) — against real corpus data and the old
//! project's known-unsolvable cascade scenario.

use binsolve_core::event::NullObserver;
use binsolve_core::grid::Cell;
use binsolve_core::parse::{parse_corpus_file, parse_line};
use binsolve_core::search::{SolveMode, SolveOutcome, solve};
use std::fs;
use std::path::PathBuf;

fn puzzle_from_rows(rows: &[&str], n: usize) -> String {
    let mut line = String::new();
    for row in rows {
        line.push_str(row);
    }
    line.push_str(&".".repeat(n * n - line.len()));
    line
}

#[test]
fn k4_cascade_scenario_from_old_readme_solves() {
    // The C# project's documented failure: rows 0 and 3 identical
    // pattern, cascade through column duos and row counts required.
    let rows = ["..011010", "..100110", "10100101", "..011010"];
    let puzzle = parse_line(&puzzle_from_rows(&rows, 8)).unwrap();
    let outcome = solve(&puzzle, SolveMode::FirstSolution, &mut NullObserver);
    assert!(
        matches!(outcome, SolveOutcome::Solved { .. }),
        "cascade puzzle must solve: {outcome:?}"
    );
}

#[test]
fn k4_cascade_wrong_branch_is_contradictory() {
    // README: if r1c1 = 0, column duos force rows 0 and 3 to become
    // identical — the whole grid is then unsolvable.
    let rows = ["..011010", ".0100110", "10100101", "..011010"];
    let puzzle = parse_line(&puzzle_from_rows(&rows, 8)).unwrap();
    let outcome = solve(&puzzle, SolveMode::FirstSolution, &mut NullObserver);
    assert!(
        matches!(outcome, SolveOutcome::Contradiction { .. }),
        "wrong cascade branch must contradict: {outcome:?}"
    );
}

#[test]
fn k5_empty_4x4_reports_multiple_fast() {
    let puzzle = parse_line(&".".repeat(16)).unwrap();
    let outcome = solve(&puzzle, SolveMode::ProveUniqueness, &mut NullObserver);
    let SolveOutcome::MultipleSolutions { first, second } = outcome else {
        panic!("empty grid has many solutions: {outcome:?}");
    };
    assert_ne!(first, second);
    assert!(first.is_complete() && second.is_complete());
}

#[test]
fn k5_two_solution_grid_reports_both() {
    // 6x6 with rows 0-4 fixed to a valid prefix leaving the last row
    // ambiguous only via full search... simplest honest construction:
    // take a solved corpus puzzle and blank a 2x2 block that admits a
    // swap without breaking any rule is NOT generally possible — so use
    // the empty 4x4 pair and assert both returned solutions validate.
    let puzzle = parse_line(&".".repeat(16)).unwrap();
    let SolveOutcome::MultipleSolutions { first, second } =
        solve(&puzzle, SolveMode::ProveUniqueness, &mut NullObserver)
    else {
        panic!("expected multiple");
    };
    use binsolve_core::region::validate_solution;
    for grid in [&first, &second] {
        assert!(validate_solution(grid, &puzzle.regions()).is_empty());
    }
}

#[test]
fn k6_contradiction_reasons_per_rule_family() {
    // Triple among the givens.
    let p = parse_line(&format!("000{}", ".".repeat(33))).unwrap();
    let SolveOutcome::Contradiction { reason } =
        solve(&p, SolveMode::FirstSolution, &mut NullObserver)
    else {
        panic!("triple givens must contradict");
    };
    assert!(reason.to_string().contains("three consecutive"), "{reason}");

    // Count overflow: row 0 has four zeros, a 6-line holds at most 3.
    let mut line = String::from("0.00.0");
    line.push_str(&".".repeat(30));
    let p = parse_line(&line).unwrap();
    let SolveOutcome::Contradiction { reason } =
        solve(&p, SolveMode::FirstSolution, &mut NullObserver)
    else {
        panic!("count overflow must contradict");
    };
    assert!(reason.to_string().contains("may hold at most"), "{reason}");

    // Duplicate complete rows.
    let mut rows = String::new();
    rows.push_str("010011");
    rows.push_str("010011");
    rows.push_str(&".".repeat(24));
    let p = parse_line(&rows).unwrap();
    let SolveOutcome::Contradiction { reason } =
        solve(&p, SolveMode::FirstSolution, &mut NullObserver)
    else {
        panic!("duplicate rows must contradict");
    };
    assert!(reason.to_string().contains("identical"), "{reason}");
}

#[test]
fn k5_every_corpus_puzzle_solves_and_proves_unique() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus");
    let mut files = Vec::new();
    collect_txt(&root, &mut files);
    files.sort();
    assert!(files.len() >= 11);
    for path in files {
        let content = fs::read_to_string(&path).unwrap();
        let (puzzle, solution) = parse_corpus_file(&content).unwrap();
        let outcome = solve(&puzzle, SolveMode::ProveUniqueness, &mut NullObserver);
        let SolveOutcome::Solved { solution: got, .. } = outcome else {
            panic!(
                "{}: expected unique solution, got {outcome:?}",
                path.display()
            );
        };
        if let Some(expected) = solution {
            assert_eq!(got, expected, "{}", path.display());
        }
    }
}

#[test]
fn k4_near_empty_adversarial_grid_solves() {
    // One given only: search must still terminate with a valid grid.
    let mut line = String::from("1");
    line.push_str(&".".repeat(35));
    let puzzle = parse_line(&line).unwrap();
    let outcome = solve(&puzzle, SolveMode::FirstSolution, &mut NullObserver);
    let SolveOutcome::Solved { solution, .. } = outcome else {
        panic!("near-empty grid must solve: {outcome:?}");
    };
    assert_eq!(solution.get(0, 0), Cell::One);
}

#[test]
fn m1_strategies_only_mode_reports_stuck() {
    let puzzle = parse_line(&".".repeat(36)).unwrap();
    let outcome = solve(&puzzle, SolveMode::StrategiesOnly, &mut NullObserver);
    assert!(
        matches!(outcome, SolveOutcome::Stuck { filled: 0, .. }),
        "{outcome:?}"
    );
}

fn collect_txt(dir: &PathBuf, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("corpus readable") {
        let path = entry.expect("entry readable").path();
        if path.is_dir() {
            collect_txt(&path, out);
        } else if path.extension().is_some_and(|e| e == "txt") {
            out.push(path);
        }
    }
}
