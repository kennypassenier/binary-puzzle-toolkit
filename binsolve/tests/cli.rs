//! L6 E2E tests: the real binary, real files, real exit codes
//! [K8-K12, K16, M1, M3]. Uses only std (T10 dependency policy).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_binsolve");

fn tmp_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli");
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn run(args: &[&str]) -> Output {
    Command::new(BIN).args(args).output().expect("binary runs")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn code(out: &Output) -> i32 {
    out.status.code().expect("process exited normally")
}

/// A real 6x6 easy puzzle and its published solution.
const EASY6: &str = "1..0....00.1.00..1......00.1...1..00";
const EASY6_SOLUTION: &str = "101010010011100101011010001101110100";

#[test]
fn k8_single_puzzle_argument_prints_canonical_line() {
    let out = run(&[EASY6]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out).trim_end(), EASY6_SOLUTION);
}

#[test]
fn k8_tagged_special_keeps_its_tag() {
    let content = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../corpus/special/8in14/bp-2524-20260808-hard.txt"),
    )
    .unwrap();
    let mut lines = content.lines();
    let puzzle = lines.next().unwrap();
    let expected = lines.next().unwrap().strip_prefix("solution:").unwrap();
    let out = run(&[puzzle]);
    assert_eq!(code(&out), 0);
    assert_eq!(stdout(&out).trim_end(), format!("8in14:{expected}"));
}

#[test]
fn k9_batch_file_maps_one_to_one_with_markers() {
    let dir = tmp_dir();
    let input = dir.join("batch.txt");
    // Solvable, contradictory (triple), malformed, solvable.
    let contradictory = format!("000{}", ".".repeat(33));
    let body = format!("{EASY6}\n{contradictory}\nnot-a-puzzle\n{EASY6}\n");
    fs::write(&input, &body).unwrap();

    let out = run(&["--file", input.to_str().unwrap()]);
    assert_eq!(code(&out), 1, "some puzzles failed");
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4, "one output line per input line");
    assert_eq!(lines[0], EASY6_SOLUTION);
    assert_eq!(lines[1], format!("#contradiction:{contradictory}"));
    assert_eq!(lines[2], "#invalid:not-a-puzzle");
    assert_eq!(lines[3], EASY6_SOLUTION);
}

/// Regression (phase 7 audit, 2026-08-28): blank lines were filtered
/// out of the input, so every later output line described a different
/// input line than its position claimed. K9 promises output line N
/// corresponds to input line N, which only holds if nothing is dropped.
#[test]
fn k9_blank_lines_keep_the_line_mapping() {
    let dir = tmp_dir();
    let input = dir.join("blanks.txt");
    fs::write(&input, format!("{EASY6}\n\n{EASY6}\n")).unwrap();

    let out = run(&["--file", input.to_str().unwrap()]);
    let text = stdout(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        3,
        "three input lines must yield three output lines, got: {text:?}"
    );
    assert_eq!(lines[0], EASY6_SOLUTION);
    assert_eq!(lines[1], "#invalid:", "the blank line keeps its slot");
    assert_eq!(lines[2], EASY6_SOLUTION);
    assert_eq!(code(&out), 1, "a blank line is a failed puzzle");
}

#[test]
fn k9_crlf_input_is_accepted() {
    let dir = tmp_dir();
    let input = dir.join("crlf.txt");
    fs::write(&input, format!("{EASY6}\r\n{EASY6}\r\n")).unwrap();
    let out = run(&["--file", input.to_str().unwrap()]);
    assert_eq!(code(&out), 0, "CRLF input must work (K14)");
    assert_eq!(stdout(&out).lines().count(), 2);
}

#[test]
fn k10_out_file_matches_stdout() {
    let dir = tmp_dir();
    let input = dir.join("in.txt");
    let output = dir.join("out.txt");
    fs::write(&input, format!("{EASY6}\n{EASY6}\n")).unwrap();

    let piped = run(&["--file", input.to_str().unwrap()]);
    let to_file = run(&[
        "--file",
        input.to_str().unwrap(),
        "--out",
        output.to_str().unwrap(),
    ]);
    assert_eq!(code(&to_file), 0);
    assert_eq!(fs::read_to_string(&output).unwrap(), stdout(&piped));
}

#[test]
fn k12_exit_codes() {
    // 0: all solved.
    assert_eq!(code(&run(&[EASY6])), 0);
    // 1: puzzle failed (contradictory givens).
    assert_eq!(code(&run(&[&format!("000{}", ".".repeat(33))])), 1);
    // 2: usage error (no arguments).
    assert_eq!(code(&run(&[])), 2);
    // 2: unreadable file.
    assert_eq!(code(&run(&["--file", "/nonexistent/puzzles.txt"])), 2);
}

#[test]
fn k16_explain_writes_steps_to_stderr_not_stdout() {
    let out = run(&["--explain", EASY6]);
    assert_eq!(code(&out), 0);
    // stdout stays canonical: exactly the solution line.
    assert_eq!(stdout(&out).trim_end(), EASY6_SOLUTION);
    let trace = String::from_utf8_lossy(&out.stderr);
    assert!(trace.contains("step 1:"), "trace missing: {trace}");
    assert!(
        trace.contains("solution found"),
        "trace missing end: {trace}"
    );
}

#[test]
fn k16_explain_to_file() {
    let dir = tmp_dir();
    let trace_file = dir.join("trace.txt");
    let out = run(&[
        &format!("--explain={}", trace_file.to_str().unwrap()),
        EASY6,
    ]);
    assert_eq!(code(&out), 0);
    assert!(out.stderr.is_empty(), "trace went to the file, not stderr");
    let trace = fs::read_to_string(&trace_file).unwrap();
    assert!(trace.contains("step 1:"));
}

#[test]
fn k16_bare_explain_never_swallows_the_puzzle_argument() {
    // Regression: with an optional value and no required equals sign,
    // clap consumed the puzzle as the trace target and the command
    // failed with "no puzzle given".
    let out = run(&["--explain", EASY6]);
    assert_eq!(code(&out), 0, "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(stdout(&out).trim_end(), EASY6_SOLUTION);
}

#[test]
fn m1_no_backtrack_reports_stuck() {
    let empty = ".".repeat(36);
    let out = run(&["--no-backtrack", &empty]);
    assert_eq!(code(&out), 1);
    assert_eq!(stdout(&out).trim_end(), format!("#stuck:{empty}"));
}

#[test]
fn k5_unique_flag_reports_multiple() {
    let empty = ".".repeat(16);
    let out = run(&["--unique", &empty]);
    assert_eq!(code(&out), 1);
    assert_eq!(stdout(&out).trim_end(), format!("#multiple:{empty}"));
}

#[test]
fn m3_check_validates_corpus_files() {
    let good = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../corpus/standard/6/bp-s6l1n1-20260812-easy.txt");
    let out = run(&["--check", "--file", good.to_str().unwrap()]);
    assert_eq!(code(&out), 0, "{}", stdout(&out));
    assert!(stdout(&out).starts_with("ok:"));

    // Corrupt the solution: swap two cells so the balance breaks.
    let dir = tmp_dir();
    let bad = dir.join("bad.txt");
    let content = fs::read_to_string(&good).unwrap();
    let mut lines = content.lines();
    let puzzle = lines.next().unwrap();
    let solution = lines.next().unwrap();
    let mut chars: Vec<char> = solution.chars().collect();
    let last = chars.len() - 1;
    chars[last] = if chars[last] == '0' { '1' } else { '0' };
    fs::write(
        &bad,
        format!("{puzzle}\n{}\n", chars.iter().collect::<String>()),
    )
    .unwrap();

    let out = run(&["--check", "--file", bad.to_str().unwrap()]);
    assert_eq!(code(&out), 1);
    assert!(stdout(&out).contains("invalid:"), "{}", stdout(&out));
}

#[test]
fn k7_error_messages_carry_remedies() {
    let out = run(&["1..0x0..1"]);
    // A single malformed puzzle is a failed puzzle, not a usage error.
    assert_eq!(code(&out), 1);
    assert_eq!(stdout(&out).trim_end(), "#invalid:1..0x0..1");

    let out = run(&[]);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("--file"),
        "usage error must suggest a remedy: {err}"
    );
}
