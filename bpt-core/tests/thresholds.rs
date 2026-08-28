//! K13 hard thresholds: the G5 promises from docs/SCOPE.md, as a
//! pass/fail test. Timing assertions only run in release builds —
//! a debug build is 10-30x slower and would fail meaninglessly.

use bpt_core::event::NullObserver;
use bpt_core::parse::parse_corpus_file;
use bpt_core::region::Puzzle;
use bpt_core::search::{SolveMode, SolveOutcome, solve};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

fn load_all() -> Vec<(String, Puzzle)> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus");
    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let content = fs::read_to_string(&path).expect("corpus file");
            let (puzzle, _) = parse_corpus_file(&content).expect("valid corpus file");
            let name = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .display()
                .to_string();
            (name, puzzle)
        })
        .collect()
}

/// Regression (M7 fuzzer): sparse grids once took seconds because DFS
/// explored branches that had already broken a rule no strategy
/// inspects (an over-filled line). The first two inputs took 2.08 s and
/// 0.56 s before partial validation was added to the search; the third
/// is the slowest input found by a full hour of solver fuzzing on the
/// fixed code, and is budgeted against the G5 bound because it sits
/// closest to it.
///
/// None of these are real puzzles — published puzzles carry enough
/// givens to be uniquely solvable. They are adversarial inputs the
/// scraper pipeline could still hand us.
#[test]
fn k13_adversarial_sparse_inputs_stay_fast() {
    let cases: [(&str, Duration); 3] = [
        // 10x10, 14 givens — solvable.
        (
            "...01...........01........1...0...0..1..................1...........0....1.........1..........0.1...",
            Duration::from_millis(100),
        ),
        // 8x8, 8 givens — contradictory, must be proven so.
        (
            ".1.......................0......0....1...........1.......1.1....",
            Duration::from_millis(100),
        ),
        // 10x10, 9 givens — slowest unit from a 7.5M-execution fuzz run.
        (
            ".1......................1.......1....................................1.......1.............00....11.",
            Duration::from_secs(1),
        ),
    ];
    for (case, budget) in cases {
        let puzzle = bpt_core::parse::parse_line(case).expect("valid input");
        let started = Instant::now();
        let _ = solve(&puzzle, SolveMode::FirstSolution, &mut NullObserver);
        let elapsed = started.elapsed();
        println!(
            "adversarial {}x{} solved in {elapsed:.1?} (budget {budget:.1?})",
            puzzle.givens.size(),
            puzzle.givens.size()
        );
        if cfg!(debug_assertions) {
            continue;
        }
        assert!(
            elapsed < budget,
            "sparse input took {elapsed:.1?}, over its {budget:.1?} budget"
        );
    }
}

#[test]
fn k13_g5_thresholds() {
    let puzzles = load_all();
    assert!(!puzzles.is_empty());

    // Worst single puzzle and the typical (median) case.
    let mut timings: Vec<(Duration, String)> = Vec::new();
    for (name, puzzle) in &puzzles {
        let started = Instant::now();
        let outcome = solve(puzzle, SolveMode::ProveUniqueness, &mut NullObserver);
        let elapsed = started.elapsed();
        assert!(
            matches!(outcome, SolveOutcome::Solved { .. }),
            "{name} must solve uniquely"
        );
        timings.push((elapsed, name.clone()));
    }
    timings.sort();
    let median = timings[timings.len() / 2].0;
    let (worst, worst_name) = timings.last().cloned().expect("non-empty");

    // 1,000 solves by cycling the corpus.
    let batch_started = Instant::now();
    for i in 0..1000 {
        let (_, puzzle) = &puzzles[i % puzzles.len()];
        let _ = solve(puzzle, SolveMode::FirstSolution, &mut NullObserver);
    }
    let batch = batch_started.elapsed();

    println!(
        "G5 measurements ({} build): worst {worst:.1?} ({worst_name}), median {median:.1?}, 1000-batch {batch:.2?}",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        }
    );

    if cfg!(debug_assertions) {
        println!("debug build — thresholds asserted in release only (cargo test --release)");
        return;
    }
    assert!(
        worst < Duration::from_secs(1),
        "G5: worst single puzzle {worst:.1?} exceeds 1 s ({worst_name})"
    );
    assert!(
        median < Duration::from_millis(50),
        "G5: median puzzle {median:.1?} exceeds 50 ms"
    );
    assert!(
        batch < Duration::from_secs(30),
        "G5: 1000-puzzle batch {batch:.2?} exceeds 30 s"
    );
}
