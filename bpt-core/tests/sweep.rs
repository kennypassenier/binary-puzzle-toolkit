//! M1 strategy-only sweep over the whole corpus: solved puzzles must
//! match published solutions; nothing may report a contradiction (all
//! corpus puzzles are valid). Run with `--nocapture` to see coverage.

use bpt_core::event::NullObserver;
use bpt_core::parse::parse_corpus_file;
use bpt_core::search::{StrategyRun, run_to_fixpoint};
use std::fs;
use std::path::PathBuf;

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

#[test]
fn m1_strategy_only_sweep_is_sound() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus");
    let mut files = Vec::new();
    collect_txt(&root, &mut files);
    files.sort();
    println!("strategy-only sweep ({} puzzles):", files.len());
    for path in files {
        let content = fs::read_to_string(&path).unwrap();
        let (puzzle, solution) = parse_corpus_file(&content).unwrap();
        let name = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .display()
            .to_string();
        match run_to_fixpoint(&puzzle, &mut NullObserver) {
            StrategyRun::Solved(grid) => {
                if let Some(expected) = &solution {
                    assert_eq!(
                        grid.to_line(),
                        expected.to_line(),
                        "{name}: solved grid differs from published solution"
                    );
                }
                println!("  solved   {name}");
            }
            StrategyRun::Stuck { grid, filled } => {
                let total = grid.size() * grid.size();
                println!(
                    "  stuck    {name} ({filled}/{total} cells, {}%)",
                    filled * 100 / total
                );
            }
            StrategyRun::Contradiction { row, col } => {
                panic!("{name}: false contradiction at r{row}c{col} — corpus puzzles are valid");
            }
        }
    }
}
