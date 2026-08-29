//! K30 benchmarks: what generation costs, per geometry and per level.
//!
//! Informational trend data. The hard pass/fail numbers from scope G8
//! live in `tests/thresholds.rs`, and the regression baseline AR25 asks
//! for lives in `benchmarks/baseline.json`.

use bpt_core::region::{PuzzleKind, Region};
use bpt_forge::carve::carve;
use bpt_forge::fill;
use bpt_forge::grade::Level;
use bpt_forge::rng;
use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn geometry(kind: &str) -> (usize, Vec<Region>) {
    match kind.parse::<usize>() {
        Ok(n) => (n, vec![Region::square(0, 0, n)]),
        Err(_) => {
            let k = PuzzleKind::from_tag(kind).expect("a known type");
            (k.grid_size(), k.regions())
        }
    }
}

/// One whole puzzle: fill, then carve. Splitting the two would hide the
/// only number a user experiences.
fn one(kind: &str, level: Level, seed: u64) {
    let (n, regions) = geometry(kind);
    let mut r = rng::stream(seed, 0, 0);
    let solution = fill::solution(n, &regions, &mut r).expect("solvable");
    black_box(carve(&solution, &regions, level, &mut r));
}

fn sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("by size");
    // 16 and up are minutes per sample under criterion's repetition, so
    // they are measured by tests/thresholds.rs instead of here.
    for kind in ["6", "8", "10", "12", "14"] {
        group.bench_function(kind, |b| b.iter(|| one(kind, Level::L4, 1)));
    }
    group.finish();
}

fn specials(c: &mut Criterion) {
    let mut group = c.benchmark_group("by type");
    for kind in ["4x6x6", "8in14"] {
        group.bench_function(kind, |b| b.iter(|| one(kind, Level::L4, 1)));
    }
    group.finish();
}

fn levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("by level (10x10)");
    for level in [Level::L1, Level::L2, Level::L3, Level::L4] {
        group.bench_function(level.name(), |b| b.iter(|| one("10", level, 1)));
    }
    group.finish();
}

criterion_group!(benches, sizes, specials, levels);
criterion_main!(benches);
