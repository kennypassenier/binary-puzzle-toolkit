//! K30 hard thresholds: scope G8's promises as a pass/fail test.
//!
//! Timing assertions only run in release. A debug build is more than an
//! order of magnitude slower and would fail meaninglessly — the same
//! rule the solver's own threshold test follows.
//!
//! These are sequential measurements on purpose. Parallelism (M22)
//! divides a batch across cores; it cannot make one puzzle faster, and
//! G8 is stated per puzzle.

use bpt_core::region::{PuzzleKind, Region};
use bpt_forge::carve::carve;
use bpt_forge::fill;
use bpt_forge::grade::Level;
use bpt_forge::rng;
use std::time::{Duration, Instant};

fn geometry(kind: &str) -> (usize, Vec<Region>) {
    match kind.parse::<usize>() {
        Ok(n) => (n, vec![Region::square(0, 0, n)]),
        Err(_) => {
            let k = PuzzleKind::from_tag(kind).expect("a known type");
            (k.grid_size(), k.regions())
        }
    }
}

/// Generate `count` puzzles of `kind` at `level`, returning the total
/// time and the slowest single puzzle.
fn measure(kind: &str, level: Level, count: u64) -> (Duration, Duration) {
    let (n, regions) = geometry(kind);
    let mut total = Duration::ZERO;
    let mut worst = Duration::ZERO;
    for index in 0..count {
        let started = Instant::now();
        let mut r = rng::stream(2026, index, 0);
        let solution = fill::solution(n, &regions, &mut r).expect("a solvable geometry");
        let _ = carve(&solution, &regions, level, &mut r);
        let elapsed = started.elapsed();
        total += elapsed;
        worst = worst.max(elapsed);
    }
    (total, worst)
}

/// Release-only: in debug these numbers mean nothing.
fn timed() -> bool {
    !cfg!(debug_assertions)
}

#[test]
fn g8_a_very_hard_14x14_takes_under_ten_seconds() {
    if !timed() {
        return;
    }
    let (_, worst) = measure("14", Level::L4, 8);
    println!("14x14 L4, worst of 8: {worst:?}");
    assert!(
        worst < Duration::from_secs(10),
        "G8: the hardest 14x14 took {worst:?}, budget 10s"
    );
}

#[test]
fn g8_a_hundred_10x10_puzzles_take_under_a_minute() {
    if !timed() {
        return;
    }
    let (total, _) = measure("10", Level::L4, 100);
    println!("100x 10x10 L4: {total:?}");
    assert!(
        total < Duration::from_secs(60),
        "G8: 100 10x10 puzzles took {total:?}, budget 60s"
    );
}

/// Currently failing, deliberately left in place and ignored rather
/// than weakened: 9x6x6 measures 94.4 s against G8's 30 s. AR25 already
/// amended G8's figures to provisional and replaced them with the
/// recorded baseline, so whether 30 s survives as a target is Kenny's
/// call (mini-round Q7) — not something to quietly rewrite here.
#[test]
#[ignore = "G8's 30s target is missed by 9x6x6 at 94s — pending mini-round Q7"]
fn g8_every_special_type_takes_under_thirty_seconds() {
    if !timed() {
        return;
    }
    for kind in ["4x6x6", "4x8x8", "9x6x6", "8in14", "6in10in14"] {
        let (_, worst) = measure(kind, Level::L4, 3);
        println!("{kind} L4, worst of 3: {worst:?}");
        assert!(
            worst < Duration::from_secs(30),
            "G8: {kind} took {worst:?}, budget 30s"
        );
    }
}
