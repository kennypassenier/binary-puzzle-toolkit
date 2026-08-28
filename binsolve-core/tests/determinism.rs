//! AR13: the core is bit-deterministic — fixed iteration order, no
//! hashing, no time dependence. Three frozen promises rest on it (K16
//! trace snapshots, trace replay, M5 parallel-equals-sequential), yet
//! nothing asserted it until the phase 7 audit pointed that out.

use binsolve_core::event::{EventLog, format_trace};
use binsolve_core::parse::parse_line;
use binsolve_core::search::{SolveMode, solve};
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
