//! K31: independent validation of generated batches.
//!
//! The generator's own claim — "this puzzle has exactly one solution" —
//! is checked by running the finished puzzles through the solver as a
//! user would: the real binary, reading the flat file the batch wrote,
//! with `--unique`. Nothing in that path shares code with carving.
//!
//! The full D1 sweep (100 puzzles per size and per special type) is
//! marked `#[ignore]` and runs in CI, because it takes minutes; the
//! quick sweep here covers every geometry with a small batch and runs
//! on every commit.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bpt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bpt"))
}

fn workdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bpt-validate-{name}"));
    let _ = fs::remove_dir_all(&dir);
    dir
}

/// Every geometry the toolkit claims to generate.
const GEOMETRIES: [&str; 13] = [
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
];

/// The subset a commit can afford. Carving cost climbs steeply with the
/// grid: measured in release, five puzzles take 0.2 s at 12x12, 3.8 s at
/// 14x14 and 11.2 s at 16x16, and these tests run against the debug
/// binary, which is slower again by more than an order of magnitude.
/// Everything above stays in the D1 sweep, which CI runs in release.
const QUICK: [&str; 5] = ["6", "8", "10", "12", "4x6x6"];

/// Generate `count` puzzles of `kind` into `dir` and prove every one of
/// them unique through the binary. Returns how many were checked.
fn generate_and_validate(dir: &Path, kind: &str, count: u64, seed: u64) -> usize {
    let forged = bpt()
        .args(["forge", "--out-dir"])
        .arg(dir)
        .args(["--kind", kind])
        .args(["--count", &count.to_string()])
        .args(["--seed", &seed.to_string()])
        .output()
        .expect("bpt runs");
    assert_eq!(
        forged.status.code(),
        Some(0),
        "forging {kind} failed: {}",
        String::from_utf8_lossy(&forged.stderr)
    );

    let validated = bpt()
        .args(["solve", "--file"])
        .arg(dir.join("puzzles.txt"))
        .arg("--unique")
        .output()
        .expect("bpt runs");
    let stdout = String::from_utf8_lossy(&validated.stdout);
    assert_eq!(
        validated.status.code(),
        Some(0),
        "{kind}: the solver refused a generated batch\n{stdout}"
    );
    let lines = stdout.lines().count();
    assert_eq!(lines, count as usize, "{kind}: one answer per puzzle");
    assert!(
        !stdout.contains("multiple"),
        "{kind}: the generator claimed a unique solution the solver did not confirm\n{stdout}"
    );
    lines
}

#[test]
fn k31_every_cheap_geometry_survives_independent_validation() {
    for (i, kind) in QUICK.iter().enumerate() {
        let dir = workdir(&format!("quick-{kind}"));
        // Two puzzles each: enough to catch a geometry whose generated
        // puzzles the solver disagrees with, cheap enough for a commit.
        assert_eq!(generate_and_validate(&dir, kind, 2, 100 + i as u64), 2);
        fs::remove_dir_all(&dir).ok();
    }
}

#[test]
fn k31_an_ambiguous_puzzle_is_caught() {
    // Sabotage: a real generated puzzle with one clue taken out. The
    // harness must fail on it, or it would pass on anything.
    let dir = workdir("sabotage");
    assert_eq!(generate_and_validate(&dir, "6", 1, 5), 1);

    let flat = dir.join("puzzles.txt");
    let original = fs::read_to_string(&flat).unwrap();
    let line = original.lines().next().unwrap();
    let position = line
        .char_indices()
        .find(|(_, c)| *c != '.')
        .map(|(i, _)| i)
        .expect("a generated puzzle has clues");
    let mut sabotaged: Vec<char> = line.chars().collect();
    sabotaged[position] = '.';
    let sabotaged: String = sabotaged.into_iter().collect();

    // Removing a clue from a minimal puzzle need not always destroy
    // uniqueness, so the assertion is on what the solver reports, not on
    // an assumption: whatever it says, it must not silently claim the
    // same clean result. Search until a removal that really is ambiguous
    // is found — one exists, or the puzzle was not minimal.
    let mut found_ambiguous = false;
    for (i, c) in line.char_indices() {
        if c == '.' {
            continue;
        }
        let mut candidate: Vec<char> = line.chars().collect();
        candidate[i] = '.';
        let candidate: String = candidate.into_iter().collect();
        let out = bpt()
            .args(["solve", &candidate, "--unique"])
            .output()
            .expect("bpt runs");
        let stdout = String::from_utf8_lossy(&out.stdout);
        if stdout.contains("multiple") {
            found_ambiguous = true;
            assert_ne!(
                out.status.code(),
                Some(0),
                "an ambiguous puzzle must not exit clean: {stdout}"
            );
            break;
        }
    }
    assert!(
        found_ambiguous,
        "no single removal from {sabotaged} produced an ambiguous puzzle — \
         a carved puzzle should be minimal enough that one does"
    );
    fs::remove_dir_all(&dir).ok();
}

/// How many puzzles D1's sweep validates per geometry.
///
/// D1 asks for 100 everywhere, and cost is what stands in the way: one
/// 16x16 takes seconds, one 20x20 a minute or two, so a hundred of those
/// would run for most of a day.
///
/// 18x18 and 20x20 were skipped entirely for a while, because the carve
/// could run without terminating — seed 2026 was still going after
/// eighty minutes. B4's node budget fixed that: every size now finishes,
/// so they are back in the sweep at a count their cost allows.
/// ↳ B4 = the deterministic node budget: a uniqueness question may cost
/// a fixed number of search steps, after which the clue is kept.
const SWEEP: [(&str, u64); 13] = [
    ("6", 100),
    ("8", 100),
    ("10", 100),
    ("12", 100),
    ("14", 100),
    ("16", 10),
    ("18", 5),
    ("20", 3),
    ("4x6x6", 100),
    ("4x8x8", 10),
    ("9x6x6", 5),
    ("8in14", 100),
    ("6in10in14", 10),
];

/// D1: the full sweep over every geometry, including the ones too
/// expensive for a commit. `cargo test --release -- --ignored` in CI.
#[test]
#[ignore = "D1's full sweep takes many minutes; CI runs it with --ignored"]
fn d1_every_geometry_validates_in_bulk() {
    let mut reduced = Vec::new();
    for (i, (kind, count)) in SWEEP.iter().enumerate() {
        let dir = workdir(&format!("d1-{kind}"));
        assert_eq!(
            generate_and_validate(&dir, kind, *count, 900 + i as u64),
            *count as usize
        );
        println!("D1: {kind} — {count} puzzles validated");
        if *count < 100 {
            reduced.push(format!("{kind} ({count})"));
        }
        fs::remove_dir_all(&dir).ok();
    }
    // Not a failure — a statement. D1's promise was 100 everywhere, and
    // anyone reading a green run deserves to know where it was fewer.
    if !reduced.is_empty() {
        println!(
            "D1: covered at fewer than 100 puzzles because generation \
             cost forbids it: {}",
            reduced.join(", ")
        );
    }
    // Every geometry the toolkit claims must appear in the sweep.
    for kind in GEOMETRIES {
        assert!(
            SWEEP.iter().any(|(k, _)| *k == kind),
            "{kind} is not in D1's sweep"
        );
    }
}
