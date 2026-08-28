//! K13 benchmarks: per-size and per-type solve time, plus the batch
//! throughput that the G5 targets are stated in. Informational trend
//! data; the hard pass/fail thresholds live in tests/thresholds.rs.

use bpt_core::event::NullObserver;
use bpt_core::parse::parse_corpus_file;
use bpt_core::region::Puzzle;
use bpt_core::search::{SolveMode, solve};
use criterion::{Criterion, criterion_group, criterion_main};
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus")
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

fn load_all() -> Vec<(String, Puzzle)> {
    let root = corpus_root();
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

fn bench_individual(c: &mut Criterion) {
    let mut group = c.benchmark_group("solve");
    for (name, puzzle) in load_all() {
        group.bench_function(name, |b| {
            b.iter(|| {
                solve(
                    black_box(&puzzle),
                    SolveMode::FirstSolution,
                    &mut NullObserver,
                )
            });
        });
    }
    group.finish();
}

fn bench_uniqueness(c: &mut Criterion) {
    let mut group = c.benchmark_group("prove_unique");
    for (name, puzzle) in load_all() {
        group.bench_function(name, |b| {
            b.iter(|| {
                solve(
                    black_box(&puzzle),
                    SolveMode::ProveUniqueness,
                    &mut NullObserver,
                )
            });
        });
    }
    group.finish();
}

/// The G5 batch target: 1,000 puzzles drawn by cycling the corpus.
fn bench_batch(c: &mut Criterion) {
    let all = load_all();
    let batch: Vec<&Puzzle> = (0..1000).map(|i| &all[i % all.len()].1).collect();
    let mut group = c.benchmark_group("batch");
    group.sample_size(10);
    group.bench_function("1000_puzzles", |b| {
        b.iter(|| {
            for puzzle in &batch {
                black_box(solve(puzzle, SolveMode::FirstSolution, &mut NullObserver));
            }
        });
    });
    group.finish();
}

criterion_group!(benches, bench_individual, bench_uniqueness, bench_batch);
criterion_main!(benches);
