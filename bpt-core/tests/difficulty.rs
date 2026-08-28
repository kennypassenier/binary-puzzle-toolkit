//! M2: difficulty grading, calibrated against binarypuzzle.com's own
//! labels. Run with --nocapture to see the calibration table.

use bpt_core::event::NullObserver;
use bpt_core::parse::parse_corpus_file;
use bpt_core::search::{Difficulty, SolveMode, SolveOutcome, grade, solve};
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

/// The site's own label, taken from the corpus filename suffix. The
/// site's "medium" maps onto our Easy band by design (see Difficulty).
fn site_label(path: &std::path::Path) -> Difficulty {
    let name = path.file_stem().unwrap().to_string_lossy().to_lowercase();
    if name.ends_with("veryhard") {
        Difficulty::VeryHard
    } else if name.ends_with("hard") {
        Difficulty::Hard
    } else {
        // easy and medium
        Difficulty::Easy
    }
}

#[test]
fn m2_grading_correlates_with_site_labels() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus");
    let mut files = Vec::new();
    collect_txt(&root, &mut files);
    files.sort();

    let mut rows = Vec::new();
    for path in &files {
        let content = fs::read_to_string(path).unwrap();
        let (puzzle, _) = parse_corpus_file(&content).unwrap();
        let SolveOutcome::Solved { stats, .. } =
            solve(&puzzle, SolveMode::FirstSolution, &mut NullObserver)
        else {
            panic!("{}: corpus puzzles solve", path.display());
        };
        rows.push((
            path.strip_prefix(&root).unwrap().display().to_string(),
            site_label(path),
            grade(&stats),
            stats,
        ));
    }

    println!(
        "{:<48} {:>10} {:>10} {:>6} {:>5} {:>7}",
        "puzzle", "site", "binsolve", "tier", "guess", "deduct"
    );
    for (name, site, ours, stats) in &rows {
        println!(
            "{:<48} {:>10} {:>10} {:>6} {:>5} {:>7}",
            name,
            site.name(),
            ours.name(),
            stats.max_tier,
            stats.guesses,
            stats.deductions
        );
    }

    // Calibration invariants:
    // 1. Site easy/medium puzzles never need more than tier 2.
    // 2. Site hard/very-hard puzzles always need more than tier 2.
    // 3. No grade is ever off by more than one band.
    let mut exact = 0;
    for (name, site, ours, _) in &rows {
        if *site == Difficulty::Easy {
            assert_eq!(
                *ours,
                Difficulty::Easy,
                "{name}: site says easy/medium, we say {}",
                ours.name()
            );
        } else {
            assert!(
                *ours > Difficulty::Easy,
                "{name}: site says {}, we say easy",
                site.name()
            );
        }
        let distance = (*ours as i32 - *site as i32).abs();
        assert!(
            distance <= 1,
            "{name}: site {} vs binsolve {} is more than one band apart",
            site.name(),
            ours.name()
        );
        if distance == 0 {
            exact += 1;
        }
    }
    println!("exact agreement: {exact}/{}", rows.len());
    assert!(
        exact * 10 >= rows.len() * 8,
        "expected at least 80% exact agreement, got {exact}/{}",
        rows.len()
    );

    // Monotonicity: the mean grade of site-veryhard puzzles must not be
    // below that of site-hard puzzles.
    let mean = |want: Difficulty| {
        let vals: Vec<u8> = rows
            .iter()
            .filter(|(_, s, _, _)| *s == want)
            .map(|(_, _, o, _)| *o as u8)
            .collect();
        if vals.is_empty() {
            return 0.0;
        }
        f64::from(vals.iter().map(|v| u32::from(*v)).sum::<u32>()) / vals.len() as f64
    };
    let hard = mean(Difficulty::Hard);
    let very = mean(Difficulty::VeryHard);
    println!("mean grade — site hard: {hard:.2}, site very hard: {very:.2}");
    assert!(
        very >= hard,
        "very hard puzzles must not grade below hard ones ({very:.2} < {hard:.2})"
    );
}
