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

/// D1: the full sweep over every geometry, including the ones too
/// expensive for a commit. `cargo test --release -- --ignored` in CI.
#[test]
#[ignore = "D1's full sweep takes minutes; CI runs it with --ignored"]
fn d1_a_hundred_puzzles_per_geometry_validate() {
    for (i, kind) in GEOMETRIES.iter().enumerate() {
        let dir = workdir(&format!("d1-{kind}"));
        assert_eq!(generate_and_validate(&dir, kind, 100, 900 + i as u64), 100);
        fs::remove_dir_all(&dir).ok();
    }
}
