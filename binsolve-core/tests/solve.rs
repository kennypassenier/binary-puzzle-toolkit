//! L4 integration tests: completeness (K4), uniqueness proof (K5),
//! contradiction reasons (K6) — against real corpus data and the old
//! project's known-unsolvable cascade scenario.

use binsolve_core::event::NullObserver;
use binsolve_core::grid::Cell;
use binsolve_core::parse::parse_line;
use binsolve_core::search::{ContradictionReason, SolveMode, SolveOutcome, solve};
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

/// K5 must reach the uniqueness SEARCH, not just the ladder. Every
/// corpus puzzle as published is ladder-solved, so `solve()` returns
/// before the DFS ever runs — the phase 7 audit found the whole "keep
/// searching past the first solution" path untested. Stripping givens
/// while uniqueness survives lands in the window where the ladder
/// stalls but exactly one solution remains, which is precisely what
/// the search has to prove. The first puzzle that reaches that window
/// is a composite type, so this also exercises DFS across overlapping
/// regions — a path no other test touches.
#[test]
fn k5_uniqueness_search_runs_and_still_proves_the_published_solution() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus");
    let mut files = Vec::new();
    collect_txt(&root, &mut files);
    files.sort();

    let mut exercised = 0;
    for path in &files {
        let content = fs::read_to_string(path).unwrap();
        let full = content.lines().next().unwrap();
        let expected = content
            .lines()
            .nth(1)
            .and_then(|l| l.strip_prefix("solution:"))
            .expect("corpus file has a solution");
        let (tag, body) = match full.split_once(':') {
            Some((t, b)) => (format!("{t}:"), b.to_string()),
            None => (String::new(), full.to_string()),
        };

        let mut grid: Vec<char> = body.chars().collect();
        let mut window = None;
        for i in 0..grid.len() {
            if grid[i] == '.' {
                continue;
            }
            let saved = grid[i];
            grid[i] = '.';
            let candidate = format!("{tag}{}", grid.iter().collect::<String>());
            let puzzle = parse_line(&candidate).unwrap();
            if !matches!(
                solve(&puzzle, SolveMode::ProveUniqueness, &mut NullObserver),
                SolveOutcome::Solved { .. }
            ) {
                grid[i] = saved; // that given was load-bearing for uniqueness
                continue;
            }
            if matches!(
                solve(&puzzle, SolveMode::StrategiesOnly, &mut NullObserver),
                SolveOutcome::Stuck { .. }
            ) {
                window = Some(candidate);
                break;
            }
        }
        let Some(stripped) = window else {
            continue;
        };

        let puzzle = parse_line(&stripped).unwrap();
        let outcome = solve(&puzzle, SolveMode::ProveUniqueness, &mut NullObserver);
        let SolveOutcome::Solved { solution, stats } = outcome else {
            panic!(
                "{}: stripped puzzle must stay uniquely solvable: {outcome:?}",
                path.display()
            );
        };
        assert!(
            stats.guesses > 0,
            "{}: this case exists to exercise the search, but it never guessed",
            path.display()
        );
        assert_eq!(
            solution.to_line(),
            expected,
            "{}: the uniqueness search found a different grid than the published solution",
            path.display()
        );
        exercised += 1;
        if exercised == 2 {
            return;
        }
    }
    panic!("no corpus puzzle reached the uniqueness search; the path stays untested");
}

/// A genuinely ambiguous grid must report BOTH solutions, not silently
/// pick one. Two 4x4 grids that differ only in a swappable pair.
#[test]
fn k5_two_solution_grid_reports_both() {
    let puzzle = parse_line("0110100101101001").unwrap();
    // Blank the whole grid except one row: several completions remain.
    let mut ambiguous = String::from("0110");
    ambiguous.push_str(&".".repeat(12));
    let puzzle2 = parse_line(&ambiguous).unwrap();
    let outcome = solve(&puzzle2, SolveMode::ProveUniqueness, &mut NullObserver);
    let SolveOutcome::MultipleSolutions { first, second } = outcome else {
        panic!("a single given row leaves several valid grids: {outcome:?}");
    };
    assert_ne!(first, second, "the two reported solutions must differ");
    use binsolve_core::region::validate_solution;
    for grid in [&first, &second] {
        assert!(
            validate_solution(grid, &puzzle2.regions()).is_empty(),
            "each reported solution must satisfy every rule"
        );
        assert_eq!(&grid.to_line()[..4], "0110", "the given row is preserved");
    }
    let _ = puzzle;
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

/// K6 promised one crafted input per rule family "asserting the
/// specific reason text". Two of the three reason variants were never
/// checked: Exhausted (search proved no assignment works) and
/// ConflictingDeductions (two sound rules demand opposite values).
#[test]
fn k6_exhausted_and_conflicting_reasons_carry_their_text() {
    // Two deductions fight over one cell: the 00 pair forces c2 = 1
    // while the 11 pair forces c2 = 0.
    let mut line = String::from("00.11.");
    line.push_str(&".".repeat(30));
    let puzzle = parse_line(&line).unwrap();
    let SolveOutcome::Contradiction { reason } =
        solve(&puzzle, SolveMode::FirstSolution, &mut NullObserver)
    else {
        panic!("conflicting deductions must contradict");
    };
    let text = reason.to_string();
    assert!(
        text.contains("forced to both 0 and 1"),
        "must name the conflict: {text}"
    );
    assert!(text.contains("r0c2"), "must name the cell: {text}");

    // Exhausted means the search refuted every assignment. It is rare
    // in practice — partial validation catches most impossible grids
    // earlier with a specific rule violation — so its wording is
    // asserted directly; that is what regresses if someone edits it.
    assert_eq!(
        ContradictionReason::Exhausted.to_string(),
        "no assignment of the open cells satisfies all rules (search exhausted)"
    );
}

/// Any Solved outcome must satisfy every rule, for every corpus-sized
/// grid the fuzzer might explore (property form of the regression).
#[test]
fn k6_solved_outcomes_are_always_valid() {
    let cases = [
        ".10.",
        "0110",
        "01..",
        "....",
        "10..01..",
        "0.1.1.0.",
        "0.0.0.0.0.0.0.0.",
        ".1.0.0.1........",
    ];
    for case in cases {
        let Ok(puzzle) = parse_line(case) else {
            continue;
        };
        if let SolveOutcome::Solved { solution, .. } =
            solve(&puzzle, SolveMode::FirstSolution, &mut NullObserver)
        {
            use binsolve_core::region::validate_solution;
            let violations = validate_solution(&solution, &puzzle.regions());
            assert!(
                violations.is_empty(),
                "{case}: solved grid violates rules: {}",
                violations[0]
            );
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
