//! M30: the restore drill.
//!
//! A batch that was generated months ago is regenerated from nothing but
//! its manifest and compared to the files on disk, byte for byte. It
//! proves two things at once: that the recorded `(seed, index, attempt)`
//! triples really do restore a corpus, and that reproducibility has not
//! broken silently since the batch was written.
//!
//! The fixture in `fixtures/restore-drill/` is committed on purpose — a
//! drill that regenerates a batch made moments earlier by the same
//! binary would pass no matter what changed.

use bpt_core::region::{PuzzleKind, Region};
use bpt_forge::batch::{self, Plan};
use bpt_forge::grade::Level;
use bpt_forge::manifest::{self, Manifest};
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .join("fixtures/restore-drill")
}

fn load(dir: &Path) -> Manifest {
    let text = fs::read_to_string(dir.join("manifest.toml")).expect("the fixture manifest");
    Manifest::from_toml(&text).expect("the fixture manifest parses")
}

/// Rebuild the plan the manifest describes. Everything needed is in the
/// manifest, which is the property the drill is really testing: a
/// manifest that cannot reconstruct its own plan cannot restore
/// anything.
fn plan_from(manifest: &Manifest) -> Plan {
    let regions = match manifest.kind.parse::<usize>() {
        Ok(n) => vec![Region::square(0, 0, n)],
        Err(_) => PuzzleKind::from_tag(&manifest.kind)
            .expect("a manifest names a known geometry")
            .regions(),
    };
    Plan::new(
        manifest.grid_size,
        regions,
        manifest.level_ceiling,
        manifest.seed,
        manifest.requested,
    )
}

#[test]
fn m30_a_stored_batch_regenerates_byte_for_byte() {
    let dir = fixture_dir();
    let manifest = load(&dir);
    assert_eq!(
        manifest.grading_version,
        manifest::GRADING_VERSION,
        "the fixture was graded by a different ladder — regrade the fixture deliberately, \
         do not weaken the drill"
    );
    let plan = plan_from(&manifest);

    for entry in &manifest.puzzles {
        let carved = batch::regenerate(&plan, entry.index, entry.attempt)
            .expect("the fixture's geometry is solvable");
        let stored = fs::read_to_string(dir.join(&entry.file))
            .unwrap_or_else(|_| panic!("{} is missing from the fixture", entry.file));
        let rebuilt = format!(
            "{}\nsolution:{}\n",
            carved.puzzle.to_line(),
            carved.solution.to_line()
        );
        assert_eq!(
            stored, rebuilt,
            "{} did not restore — reproducibility broke since the fixture was written",
            entry.file
        );
        assert_eq!(
            entry.digest,
            manifest::digest(carved.puzzle.to_line().as_str())
        );
        assert_eq!(entry.level, carved.level, "{}", entry.file);
        assert_eq!(entry.clues, carved.clues, "{}", entry.file);
    }

    // The flat file is part of the batch, so the drill restores it too.
    let flat: String = manifest
        .puzzles
        .iter()
        .map(|entry| {
            let carved = batch::regenerate(&plan, entry.index, entry.attempt).unwrap();
            format!("{}\n", carved.puzzle.to_line())
        })
        .collect();
    assert_eq!(fs::read_to_string(dir.join("puzzles.txt")).unwrap(), flat);
}

#[test]
fn m30_an_altered_manifest_makes_the_drill_fail() {
    let dir = fixture_dir();
    let mut manifest = load(&dir);
    let plan = plan_from(&manifest);

    // Exactly the failure the drill exists to catch: a manifest whose
    // triple no longer points at the file next to it.
    let entry = &mut manifest.puzzles[0];
    entry.attempt += 1;
    let carved = batch::regenerate(&plan, entry.index, entry.attempt).unwrap();
    let stored = fs::read_to_string(dir.join(&entry.file)).unwrap();
    assert_ne!(
        stored,
        format!(
            "{}\nsolution:{}\n",
            carved.puzzle.to_line(),
            carved.solution.to_line()
        ),
        "an altered triple must not restore the same puzzle"
    );

    // And a tampered seed must not quietly restore either.
    let mut wrong = plan_from(&manifest);
    wrong.seed = manifest.seed + 1;
    let elsewhere = batch::regenerate(&wrong, 0, 0).unwrap();
    assert_ne!(
        elsewhere.puzzle.to_line(),
        batch::regenerate(&plan, 0, 0).unwrap().puzzle.to_line()
    );
}

#[test]
fn m30_the_fixture_is_a_batch_the_toolkit_still_accepts() {
    // A fixture that no longer parses would make the drill vacuous.
    let dir = fixture_dir();
    let manifest = load(&dir);
    assert_eq!(manifest.completed as usize, manifest.puzzles.len());
    assert!(manifest.completed > 0, "an empty fixture proves nothing");
    for entry in &manifest.puzzles {
        let stored = fs::read_to_string(dir.join(&entry.file)).unwrap();
        let line = stored.lines().next().unwrap();
        let puzzle = bpt_core::parse::parse_line(line).expect("the fixture still parses");
        assert!(matches!(
            bpt_core::search::solve(
                &puzzle,
                bpt_core::search::SolveMode::ProveUniqueness,
                &mut bpt_core::event::NullObserver,
            ),
            bpt_core::search::SolveOutcome::Solved { .. }
        ));
    }
    let _ = Level::L4;
}
