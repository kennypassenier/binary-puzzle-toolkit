//! M7: the solver must terminate and never panic on any parseable
//! grid, and any solution it reports must satisfy every rule.

#![no_main]

use bpt_core::event::NullObserver;
use bpt_core::parse::parse_line;
use bpt_core::region::validate_solution;
use bpt_core::search::{SolveMode, SolveOutcome, solve};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(puzzle) = parse_line(text) else {
        return;
    };
    // Grids above 10x10 with many empty cells can take exponential
    // time by design; the fuzzer's job here is logic, not endurance.
    if puzzle.givens.size() > 10 {
        return;
    }
    let outcome = solve(&puzzle, SolveMode::FirstSolution, &mut NullObserver);
    match outcome {
        SolveOutcome::Solved { solution, .. } => {
            assert!(
                validate_solution(&solution, &puzzle.regions()).is_empty(),
                "reported solution violates the rules"
            );
            for r in 0..puzzle.givens.size() {
                for c in 0..puzzle.givens.size() {
                    let given = puzzle.givens.get(r, c);
                    if !given.is_empty() {
                        assert_eq!(given, solution.get(r, c), "solution changed a given");
                    }
                }
            }
        }
        SolveOutcome::MultipleSolutions { first, second } => {
            assert_ne!(first, second);
        }
        SolveOutcome::Contradiction { .. } | SolveOutcome::Stuck { .. } => {}
    }
});
