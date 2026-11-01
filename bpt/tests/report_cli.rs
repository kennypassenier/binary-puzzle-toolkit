//! feat-report-1 end to end: the real binary over real files, with the
//! exit status a caller such as a website's CI would gate on.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_bpt");

fn tmp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("report")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir");
    dir
}

fn report(args: &[&Path]) -> Output {
    Command::new(BIN)
        .arg("report")
        .args(args)
        .output()
        .expect("binary runs")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

const EASY6: &str = "1..0....00.1.00..1......00.1...1..00";

#[test]
fn feat_human_1_the_corpus_splits_into_by_hand_and_enumeration() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("../corpus");
    let out = report(&[&corpus]);
    let text = stdout(&out);
    // Four of binarypuzzle.com's very hard puzzles are only reachable
    // through line enumeration: no guessing, but not by hand either.
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.starts_with("20 puzzle(s) from 20 file(s); skipped 20 solution line(s)"));
    assert!(text.contains("solvable by a person (one solution, no guessing): 16 of 20"));
    assert!(text.contains("not solvable by a person:"));
}

#[test]
fn feat_report_1_a_puzzle_nobody_can_solve_fails_the_run_and_is_named() {
    let dir = tmp_dir("mixed");
    let file = dir.join("mixed.txt");
    let contradiction = format!("000{}", ".".repeat(33));
    fs::write(
        &file,
        format!(
            "{EASY6}\r\nsolution:101010010011100101011010001101110100\n\n{}\n{contradiction}\nnot a puzzle\n",
            ".".repeat(36)
        ),
    )
    .unwrap();
    let out = report(&[&file]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(text.contains("skipped 1 solution line(s) and 1 blank line(s)"));
    assert!(text.contains("solvable by a person (one solution, no guessing): 1 of 4"));
    let at = |line: usize| format!("{}:{line}", file.display());
    assert!(text.contains(&format!("  {}  not unique  (6x6, 0 clues)", at(4))));
    assert!(text.contains(&format!("  {}  no solution  (6x6, 3 clues)", at(5))));
    assert!(text.contains(&format!("  {}  invalid line", at(6))));
}

#[test]
fn feat_report_1_a_directory_contributes_only_its_txt_files() {
    let dir = tmp_dir("tree");
    fs::create_dir_all(dir.join("nested")).unwrap();
    fs::write(dir.join("nested/a.txt"), format!("{EASY6}\n")).unwrap();
    fs::write(dir.join("manifest.json"), "{\"not\": \"a puzzle\"}\n").unwrap();
    let out = report(&[&dir]);
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.starts_with("1 puzzle(s) from 1 file(s)"), "{text}");
}

#[test]
fn feat_report_1_no_puzzle_lines_is_a_usage_error_with_a_remedy() {
    let dir = tmp_dir("empty");
    let out = report(&[&dir]);
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("no puzzle lines found in 0 file(s)"), "{err}");
    assert!(err.contains("one puzzle per line"), "{err}");
}

#[test]
fn feat_human_1_steps_lists_every_step_with_its_reason() {
    let out = Command::new(BIN)
        .args(["steps", EASY6])
        .output()
        .expect("binary runs");
    let text = stdout(&out);
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert!(text.contains("   1. "));
    assert!(text.contains("[Pair]"));
    assert!(text.trim_end().ends_with("steps"));
    assert!(text.contains("solved by hand in "));
    // Every printed step carries a technique and a reason.
    let steps: Vec<&str> = text.lines().filter(|l| l.contains(". r")).collect();
    assert!(!steps.is_empty());
    assert!(steps.iter().all(|l| l.contains('[') && l.contains("  ")));
}

#[test]
fn feat_human_1_steps_exits_1_when_the_puzzle_is_not_solvable_by_hand() {
    let out = Command::new(BIN)
        .args(["steps", &".".repeat(36)])
        .output()
        .expect("binary runs");
    assert_eq!(out.status.code(), Some(1));
    assert!(stdout(&out).contains("NOT solvable by hand"));
}

#[test]
fn feat_human_1_every_variant_puzzle_is_solvable_by_hand() {
    // variants/<name>.txt belongs to geometries/<name>.toml: window
    // pyramids, where every square the blocks can form is a region.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut checked = 0;
    for entry in fs::read_dir(root.join("variants")).unwrap() {
        let file = entry.unwrap().path();
        let name = file.file_stem().unwrap().to_string_lossy().into_owned();
        let geometry = root.join("geometries").join(format!("{name}.toml"));
        let out = Command::new(BIN)
            .arg("report")
            .arg("--geometry")
            .arg(&geometry)
            .arg(&file)
            .output()
            .expect("binary runs");
        let text = stdout(&out);
        assert_eq!(out.status.code(), Some(0), "{name}: {text}");
        assert!(text.contains("of 10\n"), "{name}: {text}");
        checked += 1;
    }
    assert_eq!(checked, 5);
}
