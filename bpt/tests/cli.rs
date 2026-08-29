//! L6 E2E tests: the real binary, real files, real exit codes
//! [K8-K12, K16, M1, M3]. Uses only std (T10 dependency policy).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_bpt");

fn tmp_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli");
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn run(args: &[&str]) -> Output {
    // Every solving invocation now goes through the `solve` subcommand.
    let mut all = vec!["solve"];
    all.extend_from_slice(args);
    Command::new(BIN).args(&all).output().expect("binary runs")
}

/// For the cases that must exercise the bare command itself.
fn run_raw(args: &[&str]) -> Output {
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
    // 2: usage error (no arguments at all).
    assert_eq!(code(&run_raw(&[])), 2);
    // 2: the solve subcommand without a puzzle.
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

/// The toolkit's whole point in one test: the generator makes a puzzle,
/// the solver is handed it with no other information, and must find
/// exactly the intended solution as the only one.
#[test]
fn k29_forge_output_feeds_straight_back_into_solve() {
    let dir = tmp_dir();
    let generated = dir.join("generated.txt");

    let out = run_raw(&[
        "forge",
        "--kind",
        "6",
        "--count",
        "3",
        "--seed",
        "2026",
        "--with-solutions",
        "--out",
        generated.to_str().unwrap(),
    ]);
    assert_eq!(code(&out), 0, "{}", String::from_utf8_lossy(&out.stderr));

    let text = fs::read_to_string(&generated).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 6, "three puzzles plus three solutions");

    for pair in lines.chunks(2) {
        let (puzzle, solution) = (pair[0], pair[1].strip_prefix("solution:").unwrap());
        assert!(
            puzzle.contains('.'),
            "a generated puzzle must have empty cells: {puzzle}"
        );

        // The solver gets the puzzle alone and must prove it unique.
        let solved = run(&["--unique", puzzle]);
        assert_eq!(code(&solved), 0, "generated puzzle must solve uniquely");
        assert_eq!(
            stdout(&solved).trim_end(),
            solution,
            "the proven solution must be the one the generator carved from"
        );
    }
}

#[test]
fn k29_forge_rejects_an_unusable_type_with_a_remedy() {
    let out = run_raw(&["forge", "--kind", "7"]);
    assert_eq!(code(&out), 2);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("even size"), "must name the remedy: {err}");

    let out = run_raw(&["forge", "--kind", "nonsense"]);
    assert_eq!(code(&out), 2);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("4x6x6"), "must list the valid tags: {err}");
}

/// B5: a batch manifest names the build that wrote it, so the revision
/// has to be in the binary for the manifest to be able to record it.
#[test]
fn b5_version_carries_the_git_revision() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_bpt"))
        .arg("--version")
        .output()
        .expect("bpt runs");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains(env!("CARGO_PKG_VERSION")), "{text}");
    assert!(
        text.contains('(') && text.contains(')'),
        "the revision belongs in --version: {text}"
    );
    // Outside a git checkout the stamp reads "unknown"; inside one it is
    // a real short hash. Both are acceptable, an empty stamp is not.
    let rev = text
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(rev, _)| rev)
        .expect("a parenthesised revision");
    assert!(!rev.trim().is_empty(), "the revision must not be blank");
}

/// M25: the geometry inspector, which the help text has promised since
/// the merge but which nothing exposed until now.
#[test]
fn m25_inspect_renders_a_builtin_and_a_file() {
    let builtin = std::process::Command::new(env!("CARGO_BIN_EXE_bpt"))
        .args(["inspect", "4x6x6"])
        .output()
        .expect("bpt runs");
    assert_eq!(builtin.status.code(), Some(0));
    let text = String::from_utf8_lossy(&builtin.stdout);
    assert!(text.contains("12x12"), "{text}");
    assert!(text.contains("5 region"), "{text}");
    assert!(text.contains("Verdict:"), "{text}");

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("geometries/8in12in16.toml");
    let file = std::process::Command::new(env!("CARGO_BIN_EXE_bpt"))
        .arg("inspect")
        .arg(&path)
        .output()
        .expect("bpt runs");
    assert_eq!(file.status.code(), Some(0));
    let text = String::from_utf8_lossy(&file.stdout);
    assert!(
        text.contains("16x16"),
        "an invented type inspects too: {text}"
    );
}

#[test]
fn m25_inspect_names_what_it_could_not_find() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_bpt"))
        .args(["inspect", "9x9x9"])
        .output()
        .expect("bpt runs");
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not a geometry file, a built-in type, or a composed"),
        "and says all three ways a geometry can be given: {stderr}"
    );
}

/// M25/S6b: inspect resolves a composed name the same way forge does.
/// Found in the Phase 7 sweep: `forge --kind 4x6x6in16` worked while
/// `inspect 4x6x6in16` said the name did not exist.
#[test]
fn m25_inspect_understands_a_composed_name() {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_bpt"))
        .args(["inspect", "4x6x6in16"])
        .output()
        .expect("bpt runs");
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("16x16"), "{text}");
    assert!(
        text.contains("6 region"),
        "the 12x12, four 6x6 and the whole grid: {text}"
    );

    // And a name outside the grammar still fails, naming all three ways
    // a geometry can be given.
    let bad = std::process::Command::new(env!("CARGO_BIN_EXE_bpt"))
        .args(["inspect", "4x6x8"])
        .output()
        .expect("bpt runs");
    assert_eq!(bad.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&bad.stderr);
    assert!(stderr.contains("composed name"), "{stderr}");
}
