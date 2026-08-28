//! K16/AR9 property: replaying a recorded event log from the givens
//! reproduces exactly the solution the solver returned — including for
//! puzzles that need guessing, where refuted branches must be undone.

use bpt_core::event::EventLog;
use bpt_core::parse::{parse_corpus_file, parse_line};
use bpt_core::region::validate_solution;
use bpt_core::search::{SolveMode, SolveOutcome, SolveStats, solve};
use bpt_tui::replay::Replay;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

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

/// Replay `line` and return (final grid, guesses seen).
fn replay_final(line: &str) -> (String, usize) {
    let puzzle = parse_line(line).expect("valid puzzle");
    let mut log = EventLog::default();
    let outcome = solve(&puzzle, SolveMode::FirstSolution, &mut log);
    let (stats, solved) = match &outcome {
        SolveOutcome::Solved { stats, .. } => (*stats, true),
        _ => (SolveStats::default(), false),
    };
    let mut replay = Replay::new(
        "property".into(),
        &puzzle,
        log.events,
        stats,
        Duration::from_micros(1),
        solved,
    );
    replay.jump_to_end();
    (replay.grid().to_line(), stats.guesses)
}

#[test]
fn k16_replaying_a_trace_reproduces_the_solution_on_the_whole_corpus() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus");
    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();
    assert!(files.len() >= 20, "corpus shrank: {} files", files.len());

    for path in &files {
        let content = fs::read_to_string(path).unwrap();
        let (puzzle, expected) = parse_corpus_file(&content).unwrap();
        let line = content.lines().next().unwrap();
        let (replayed, _) = replay_final(line);
        if let Some(expected) = expected {
            assert_eq!(
                replayed,
                expected.to_line(),
                "{}: replayed grid differs from the published solution",
                path.display()
            );
        }
        let _ = puzzle;
    }
}

#[test]
fn k16_replay_is_correct_for_puzzles_that_need_guessing() {
    // Sparse grids the strategy ladder cannot finish, so the log
    // contains guesses and backtracks — the case that used to leave
    // stale cells on screen.
    let cases = [
        ".".repeat(16),
        ".".repeat(36),
        format!("1{}", ".".repeat(35)),
        "...01...........01........1...0...0..1..................1...........0....1.........1..........0.1...".to_string(),
    ];
    let mut saw_guessing = 0;
    for case in &cases {
        let puzzle = parse_line(case).expect("valid puzzle");
        let mut log = EventLog::default();
        let outcome = solve(&puzzle, SolveMode::FirstSolution, &mut log);
        let SolveOutcome::Solved { solution, stats } = outcome else {
            continue;
        };
        if stats.guesses > 0 {
            saw_guessing += 1;
        }
        let mut replay = Replay::new(
            "guessing".into(),
            &puzzle,
            log.events,
            stats,
            Duration::from_micros(1),
            true,
        );
        replay.jump_to_end();
        assert_eq!(
            replay.grid().to_line(),
            solution.to_line(),
            "{case}: replay does not end at the solver's own solution"
        );
        assert!(
            validate_solution(replay.grid(), &puzzle.regions()).is_empty(),
            "{case}: the replayed final grid breaks the rules"
        );
    }
    assert!(
        saw_guessing >= 2,
        "at least two of these grids must require guessing, saw {saw_guessing}"
    );
}
