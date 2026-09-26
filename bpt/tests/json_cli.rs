//! feat-json-1 end to end: `bpt solve --json` keeps the one-line-per-
//! puzzle contract and gives every line the same shape.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_bpt");
const EASY6: &str = "1..0....00.1.00..1......00.1...1..00";
const EASY6_SOLUTION: &str = "101010010011100101011010001101110100";

fn solve(args: &[&str]) -> Output {
    Command::new(BIN)
        .arg("solve")
        .args(args)
        .output()
        .expect("binary runs")
}

fn objects(out: &Output) -> Vec<serde_json::Value> {
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| serde_json::from_str(l).expect("every line is one JSON object"))
        .collect()
}

#[test]
fn feat_json_1_a_solved_puzzle_carries_its_solution_and_difficulty() {
    let out = solve(&["--json", EASY6]);
    assert_eq!(out.status.code(), Some(0));
    let o = &objects(&out)[0];
    assert_eq!(o["status"], "solved");
    assert_eq!(o["puzzle"], EASY6);
    assert_eq!(o["solution"], EASY6_SOLUTION);
    assert_eq!(o["difficulty"], "easy");
    assert!(o["ms"].is_number());
    assert!(o.get("trace").is_none(), "no trace unless --explain asks");
}

#[test]
fn feat_json_1_a_batch_maps_line_for_line_with_every_field_present() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("json");
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("batch.txt");
    fs::write(
        &file,
        format!("{EASY6}\nnot a puzzle\n000{}\n", ".".repeat(33)),
    )
    .unwrap();
    let out = solve(&["--json", "--file", file.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(1), "failures keep exit 1");
    let all = objects(&out);
    let status: Vec<&str> = all.iter().map(|o| o["status"].as_str().unwrap()).collect();
    assert_eq!(status, ["solved", "invalid", "contradiction"]);
    for o in &all {
        for key in ["puzzle", "status", "solution", "difficulty", "reason", "ms"] {
            assert!(o.get(key).is_some(), "{key} missing in {o}");
        }
    }
    assert!(
        all[1]["reason"]
            .as_str()
            .unwrap()
            .contains("invalid character")
    );
    assert!(
        all[2]["reason"]
            .as_str()
            .unwrap()
            .contains("three consecutive")
    );
}

#[test]
fn feat_json_1_explain_adds_the_trace_to_the_object() {
    let out = solve(&["--json", "--explain", EASY6]);
    let o = &objects(&out)[0];
    let trace = o["trace"].as_array().expect("trace is a list of steps");
    assert!(trace[0].as_str().unwrap().starts_with("step 1:"));
}
