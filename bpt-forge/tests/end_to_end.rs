//! The generator's whole promise in one place: a generated puzzle is
//! uniquely solvable, its clues agree with its solution, and it lands on
//! the difficulty that was asked for — verified through the solver, not
//! through the generator's own bookkeeping.

use bpt_core::event::NullObserver;
use bpt_core::region::{Puzzle, Region, validate_givens, validate_solution};
use bpt_core::search::{SolveMode, SolveOutcome, solve};
use bpt_forge::carve::carve;
use bpt_forge::fill;
use bpt_forge::grade::Level;
use bpt_forge::rng::stream;

fn geometry(tag: &str) -> (usize, Vec<Region>) {
    match tag {
        "6" => (6, vec![Region::square(0, 0, 6)]),
        "8" => (8, vec![Region::square(0, 0, 8)]),
        "4x6x6" => (
            12,
            vec![
                Region::square(0, 0, 6),
                Region::square(0, 6, 6),
                Region::square(6, 0, 6),
                Region::square(6, 6, 6),
                Region::square(0, 0, 12),
            ],
        ),
        "8in14" => (14, vec![Region::square(3, 3, 8), Region::square(0, 0, 14)]),
        other => panic!("unknown geometry {other}"),
    }
}

#[test]
fn k21_generated_puzzles_survive_independent_verification() {
    // Carving proves uniqueness once per removed clue, so cost grows
    // with area: measured in release, a 6x6 carves in 0.7 ms and a
    // 4x6x6 in 250 ms — but a debug build is 10-30x slower, which turns
    // the composite cases into minutes. The small geometries always run;
    // the large ones assert in release, where they are cheap.
    let mut cases: Vec<(&str, std::ops::Range<u64>)> = vec![("6", 0..6u64), ("8", 0..4)];
    if cfg!(debug_assertions) {
        println!("debug build — composite geometries are checked by `cargo test --release`");
    } else {
        cases.push(("4x6x6", 0..2));
        cases.push(("8in14", 0..1));
    }

    for (tag, seeds) in cases {
        let (n, regions) = geometry(tag);
        for seed in seeds {
            let mut rng = stream(2026, seed, 0);
            let solution = fill::solution(n, &regions, &mut rng)
                .unwrap_or_else(|| panic!("{tag} seed {seed}: geometry must be fillable"));
            let out = carve(&solution, &regions, Level::L4, &mut rng);

            // 1 · the solution the generator started from is itself valid
            assert!(
                validate_solution(&out.solution, &regions).is_empty(),
                "{tag} seed {seed}: generated solution breaks a rule"
            );
            // 2 · every remaining clue agrees with it
            assert!(
                validate_givens(&out.puzzle, &out.solution).is_empty(),
                "{tag} seed {seed}: a clue contradicts the solution"
            );
            // 3 · the solver, told nothing, finds exactly that solution
            let puzzle = Puzzle::custom(out.puzzle.clone(), regions.clone());
            let outcome = solve(&puzzle, SolveMode::ProveUniqueness, &mut NullObserver);
            let SolveOutcome::Solved {
                solution: found, ..
            } = outcome
            else {
                panic!("{tag} seed {seed}: not uniquely solvable: {outcome:?}");
            };
            assert_eq!(
                found, out.solution,
                "{tag} seed {seed}: solver found a different grid than the generator intended"
            );
            // 4 · it actually removed something
            assert!(
                out.clues < n * n,
                "{tag} seed {seed}: nothing was carved away"
            );
        }
    }
}

#[test]
fn k25_targeting_a_level_produces_that_level_or_easier() {
    let (n, regions) = geometry("6");
    for ceiling in [Level::L1, Level::L2, Level::L3] {
        for seed in 0..4u64 {
            let mut rng = stream(77, seed, 0);
            let solution = fill::solution(n, &regions, &mut rng).unwrap();
            let out = carve(&solution, &regions, ceiling, &mut rng);
            assert!(
                out.level <= ceiling,
                "asked for at most {}, got {} (seed {seed})",
                ceiling.name(),
                out.level.name()
            );
        }
    }
}
