//! AR25: the performance baseline and its regression guard.
//!
//! Scope G8's original figures were amended to provisional precisely
//! because nobody had measured a carve loop yet. This is the mechanism
//! that replaced them: per (geometry, level), record the median and p95
//! over a number of seeds, commit it, and fail when p95 drifts past
//! 1.5x the recorded value.
//!
//! Sample counts differ by geometry on purpose. Twenty seeds of a 6x6
//! costs milliseconds; twenty seeds of a 9x6x6 costs half an hour. The
//! count is recorded next to each number so nobody reads a three-sample
//! p95 as if it were a twenty-sample one.

use bpt_core::region::{PuzzleKind, Region};
use bpt_forge::carve::carve;
use bpt_forge::fill;
use bpt_forge::grade::Level;
use bpt_forge::rng;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Entry {
    geometry: String,
    level: String,
    samples: u64,
    /// Absent when the geometry could not be measured at all. A missing
    /// number is a finding, not an omission, and `note` says which.
    median_ms: Option<f64>,
    p95_ms: Option<f64>,
    /// Whether the regression guard re-measures this entry. The
    /// expensive geometries are measured on request, not on every run.
    in_ci: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

const ALL_LEVELS: &[&str] = &["L1", "L2", "L3", "L4"];

fn baseline_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../benchmarks/baseline.json")
}

fn geometry(kind: &str) -> (usize, Vec<Region>) {
    match kind.parse::<usize>() {
        Ok(n) => (n, vec![Region::square(0, 0, n)]),
        Err(_) => {
            let k = PuzzleKind::from_tag(kind).expect("a known type");
            (k.grid_size(), k.regions())
        }
    }
}

fn level_of(name: &str) -> Level {
    match name {
        "L1" => Level::L1,
        "L2" => Level::L2,
        "L3" => Level::L3,
        _ => Level::L4,
    }
}

/// How long one sample may take before it is written off. Recording is
/// not a loop the generator runs, so a wall-clock cap here does not
/// violate AR26's "deterministic work units, never wall-clock" — it
/// bounds the *measurement*, and an unbounded carve is exactly what it
/// has to be able to survive measuring.
const SAMPLE_TIMEOUT: Duration = Duration::from_secs(150);

/// What one measurement produced.
struct Samples {
    /// Milliseconds for the samples that finished, sorted.
    times: Vec<f64>,
    /// How many ran past the cap. A carve has no upper bound on large
    /// grids, so this is a normal outcome, not an error.
    unfinished: u64,
}

/// Measure `count` puzzles, giving up on any single one that runs past
/// the cap.
///
/// The straggler is left running on its thread: a Rust thread cannot be
/// killed, and the alternative — waiting for it — is the hang this
/// exists to avoid. The test process exits when it is done, taking the
/// thread with it.
fn samples(kind: &str, level: Level, count: u64) -> Samples {
    let (n, regions) = geometry(kind);
    let mut times = Vec::new();
    let mut unfinished = 0;
    for index in 0..count {
        let (tx, rx) = mpsc::channel();
        let regions = regions.clone();
        let kind = kind.to_string();
        std::thread::spawn(move || {
            let started = Instant::now();
            let mut r = rng::stream(2026, index, 0);
            let solution = fill::solution(n, &regions, &mut r)
                .unwrap_or_else(|| panic!("{kind} has no solution at all"));
            let _ = carve(&solution, &regions, level, &mut r);
            // The receiver is gone once the cap fired; nobody is left to
            // care about the result, so a failed send is expected.
            let _ = tx.send(started.elapsed().as_secs_f64() * 1000.0);
        });
        match rx.recv_timeout(SAMPLE_TIMEOUT) {
            Ok(ms) => times.push(ms),
            Err(_) => unfinished += 1,
        }
    }
    times.sort_by(|a, b| a.partial_cmp(b).expect("no NaN from a clock"));
    Samples { times, unfinished }
}

fn median(sorted: &[f64]) -> f64 {
    let mid = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

/// Nearest-rank p95. With three samples that is simply the slowest,
/// which is why the sample count travels with the number.
fn p95(sorted: &[f64]) -> f64 {
    let rank = ((sorted.len() as f64) * 0.95).ceil() as usize;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn load() -> Vec<Entry> {
    let text = std::fs::read_to_string(baseline_path()).expect("benchmarks/baseline.json exists");
    serde_json::from_str(&text).expect("the baseline parses")
}

#[test]
fn ar25_the_baseline_covers_every_geometry() {
    let baseline = load();
    for kind in [
        "6",
        "8",
        "10",
        "12",
        "14",
        "16",
        "18",
        "20",
        "4x6x6",
        "4x8x8",
        "9x6x6",
        "8in14",
        "6in10in14",
    ] {
        assert!(
            baseline.iter().any(|e| e.geometry == kind),
            "{kind} has no recorded baseline — a geometry nobody measured is a geometry nobody \
             notices getting slower"
        );
    }
    for entry in &baseline {
        match (entry.median_ms, entry.p95_ms) {
            (Some(median), Some(p95)) => {
                assert!(entry.samples > 0, "{} has no samples", entry.geometry);
                assert!(p95 >= median, "{}: p95 below median", entry.geometry);
            }
            (None, None) => assert!(
                entry.note.is_some(),
                "{} has no numbers and no explanation — an unmeasured geometry has to say why",
                entry.geometry
            ),
            _ => panic!("{}: half a measurement", entry.geometry),
        }
        assert!(
            !entry.in_ci || entry.p95_ms.is_some(),
            "{}: guarded but unmeasured",
            entry.geometry
        );
    }
}

#[test]
fn ar25_performance_has_not_regressed() {
    if cfg!(debug_assertions) {
        return;
    }
    for entry in load().iter().filter(|e| e.in_ci) {
        let recorded = entry.p95_ms.expect("a guarded entry is measured");
        let measurement = samples(&entry.geometry, level_of(&entry.level), entry.samples);
        assert_eq!(
            measurement.unfinished, 0,
            "{} {}: a guarded geometry stopped terminating",
            entry.geometry, entry.level
        );
        let measured = p95(&measurement.times);
        println!(
            "{} {}: p95 {measured:.1}ms against a recorded {recorded:.1}ms",
            entry.geometry, entry.level
        );
        assert!(
            measured <= recorded * 1.5,
            "{} {}: p95 is {measured:.1}ms, more than 1.5x the recorded {:.1}ms. \
             Either something got slower, or the baseline needs re-recording deliberately",
            entry.geometry,
            entry.level,
            recorded
        );
    }
}

/// Regenerate the baseline: `cargo test --release -p bpt-forge --test
/// baseline -- --ignored --nocapture > benchmarks/baseline.json`.
#[test]
#[ignore = "recording the baseline takes a long time; run it deliberately"]
fn record_the_baseline() {
    // (geometry, samples, levels). Two things drive the sample count
    // down: size, and — counter-intuitively — asking for an *easier*
    // level. At L4 the ceiling can reject nothing, so the carve skips
    // the strategy ladder entirely; at L1 to L3 it runs on every
    // candidate, which on a 16x16 or larger costs far more than the
    // deeper carve does. The large geometries are therefore recorded at
    // L4 only, which is also the level the regression guard uses.
    let plan: [(&str, u64, &[&str]); 13] = [
        ("6", 20, ALL_LEVELS),
        ("8", 20, ALL_LEVELS),
        ("10", 20, ALL_LEVELS),
        ("12", 20, ALL_LEVELS),
        ("14", 10, ALL_LEVELS),
        ("16", 3, &["L4"]),
        ("18", 3, &["L4"]),
        ("20", 3, &["L4"]),
        ("4x6x6", 20, ALL_LEVELS),
        ("4x8x8", 3, &["L4"]),
        ("9x6x6", 2, &["L4"]),
        ("8in14", 10, ALL_LEVELS),
        ("6in10in14", 3, &["L4"]),
    ];
    let mut entries = Vec::new();
    for (kind, count, levels) in plan {
        for level in levels {
            let level = *level;
            let measurement = samples(kind, level_of(level), count);
            let finished = measurement.times.len();
            let note = (measurement.unfinished > 0).then(|| {
                format!(
                    "{} of {count} samples ran past {}s without finishing (see B4)",
                    measurement.unfinished,
                    SAMPLE_TIMEOUT.as_secs()
                )
            });
            entries.push(Entry {
                geometry: kind.to_string(),
                level: level.to_string(),
                samples: finished as u64,
                median_ms: (finished > 0)
                    .then(|| (median(&measurement.times) * 10.0).round() / 10.0),
                p95_ms: (finished > 0).then(|| (p95(&measurement.times) * 10.0).round() / 10.0),
                note,
                // Only the affordable geometries are guarded, and only
                // at L4: re-measuring a 20x20 on every CI run would cost
                // more than the guard is worth. A geometry with an
                // unfinished sample is never guarded — the guard would
                // then be waiting on the same unbounded carve.
                in_ci: count >= 10 && level == "L4" && measurement.unfinished == 0,
            });
            eprintln!("recorded {kind} {level}");
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&entries).expect("serializes")
    );
}
