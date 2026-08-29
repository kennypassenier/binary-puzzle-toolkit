//! K28: two types binarypuzzle.com does not publish, defined purely as
//! geometry files, generated and solved end to end.
//!
//! They emit **no** tag. The line format resolves a tag through a fixed
//! vocabulary, so a `4x10x10:` prefix would produce files nothing can
//! read; extending that vocabulary is K28's own tag mini-round and has
//! not been put to Kenny yet. Until it is, `--geometry` is how the two
//! halves agree on the regions, which is enough to prove the types work.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bpt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bpt"))
}

fn geometry(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root")
        .join("geometries")
        .join(format!("{name}.toml"))
}

/// The invented types that exist as geometry files, with their sizes.
const INVENTED: [(&str, usize); 3] = [("4x10x10", 20), ("8in12in16", 16), ("4x6x6in16", 16)];

/// The invented types whose names the grammar can read, so they need no
/// geometry file to travel: the tag itself describes the layout.
///
/// Split by cost. The grammar does not care how big a grid is, so the
/// small names prove it on every commit and the large ones — the types
/// actually adopted — are checked in release by CI.
const COMPOSED_CHEAP: [(&str, usize); 3] = [("4x4x4", 8), ("4in8", 8), ("4x4x4in12", 12)];
const COMPOSED: [(&str, usize); 3] = [("4x10x10", 20), ("8in12in16", 16), ("4x6x6in16", 16)];

/// The end-to-end run is release-only: a 20x20 in a debug build takes
/// minutes per puzzle, so this would make every commit unusable. CI runs
/// it with `--ignored` in release, alongside D1's sweep.
#[test]
#[ignore = "a 16x16 and a 20x20 are minutes each in debug; CI runs this in release"]
fn k28_both_invented_types_generate_and_solve_end_to_end() {
    for (name, size) in INVENTED {
        let out = std::env::temp_dir().join(format!("bpt-invented-{name}.txt"));
        // L2 rather than L4: carving all the way down on a 16x16 or a
        // 20x20 takes minutes, and what this test is about is that the
        // type works, not how deep it carves.
        let forged = bpt()
            .args(["forge", "--geometry"])
            .arg(geometry(name))
            .args(["--count", "2", "--seed", "3", "--level", "L2", "--out"])
            .arg(&out)
            .output()
            .expect("bpt runs");
        assert_eq!(
            forged.status.code(),
            Some(0),
            "{name} did not generate: {}",
            String::from_utf8_lossy(&forged.stderr)
        );

        let text = fs::read_to_string(&out).expect("the generated file");
        for line in text.lines() {
            assert!(
                !line.contains(':'),
                "{name} emitted a tag the reader has no vocabulary for: {line}"
            );
            assert_eq!(line.len(), size * size, "{name}: wrong grid length");
        }

        // Solved through the binary, with uniqueness proven — the same
        // bar the five published types are held to.
        let solved = bpt()
            .args(["solve", "--file"])
            .arg(&out)
            .arg("--geometry")
            .arg(geometry(name))
            .arg("--unique")
            .output()
            .expect("bpt runs");
        let stdout = String::from_utf8_lossy(&solved.stdout);
        assert_eq!(
            solved.status.code(),
            Some(0),
            "{name} did not solve: {stdout}{}",
            String::from_utf8_lossy(&solved.stderr)
        );
        assert_eq!(stdout.lines().count(), 2);
        assert!(
            !stdout.contains("multiple") && !stdout.contains("invalid"),
            "{name}: {stdout}"
        );
        fs::remove_file(&out).ok();
    }
}

#[test]
#[ignore = "generates a 16x16; CI runs this in release"]
fn k28_an_invented_type_without_its_geometry_solves_as_something_else() {
    // The reason the tag question matters, stated as a test: with no tag
    // and no geometry file, the line parses as a plain n×n, and the
    // extra regions simply are not enforced. Nothing crashes, which is
    // exactly why this must not be left implicit.
    let out = std::env::temp_dir().join("bpt-invented-untagged.txt");
    let forged = bpt()
        .args(["forge", "--geometry"])
        .arg(geometry("8in12in16"))
        .args(["--count", "1", "--seed", "3", "--level", "L2", "--out"])
        .arg(&out)
        .output()
        .expect("bpt runs");
    assert_eq!(forged.status.code(), Some(0));

    let without = bpt()
        .args(["solve", "--file"])
        .arg(&out)
        .arg("--unique")
        .output()
        .expect("bpt runs");
    let plain = String::from_utf8_lossy(&without.stdout);

    let with = bpt()
        .args(["solve", "--file"])
        .arg(&out)
        .arg("--geometry")
        .arg(geometry("8in12in16"))
        .arg("--unique")
        .output()
        .expect("bpt runs");
    let full = String::from_utf8_lossy(&with.stdout);

    // Read as a plain 16x16 the puzzle has more than one solution: the
    // inner regions are what pin it down. That difference is the whole
    // argument for giving invented types a tag one day.
    assert_ne!(
        plain.trim(),
        full.trim(),
        "if these agreed, the inner regions would not be doing anything"
    );
    fs::remove_file(&out).ok();
}

/// Cheap enough for every commit: the files themselves are well formed,
/// describe the types they claim to, and are structurally valid. What
/// they generate is checked by the release-mode tests above.
#[test]
fn k28_the_invented_geometries_are_valid() {
    for (name, size) in INVENTED {
        let text = fs::read_to_string(geometry(name)).expect("the geometry file");
        let geometry = bpt_forge::geometry::Geometry::from_toml(&text)
            .unwrap_or_else(|e| panic!("{name} is not a usable geometry: {e}"));
        assert_eq!(geometry.size, size);
        assert_eq!(geometry.tag.as_deref(), Some(name));
        // More than one region is what makes it a composite at all, and
        // one of them must cover the whole grid or the outer rules would
        // go unenforced.
        assert!(geometry.regions.len() > 1, "{name} is not a composite");
        let regions = geometry.to_regions();
        assert!(
            regions
                .iter()
                .any(|r| r.rows == size && r.cols == size && r.row == 0 && r.col == 0),
            "{name} has no whole-grid region"
        );
        // A type nobody publishes is still a type the reader knows
        // nothing about by name.
        assert!(
            bpt_core::region::PuzzleKind::from_tag(name).is_none(),
            "{name} is in the built-in vocabulary after all — the tag mini-round happened \
             without this test being updated"
        );
    }
}

#[test]
fn k28_a_geometry_that_does_not_fit_the_grid_is_refused() {
    // The regions of a 16x16 geometry index past the end of a 6x6 grid.
    // Found by trying it: it used to panic.
    let file = std::env::temp_dir().join("bpt-invented-mismatch.txt");
    fs::write(&file, "1.0..0..0.1..1.0.0..0.1..1.0..1.0..0\n").expect("scratch file");
    let out = bpt()
        .args(["solve", "--file"])
        .arg(&file)
        .arg("--geometry")
        .arg(geometry("8in12in16"))
        .output()
        .expect("bpt runs");

    assert_eq!(
        out.status.code(),
        Some(1),
        "a mismatch is a failed puzzle, not a crash"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("#invalid:"), "{stdout}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("panicked"), "{stderr}");
    fs::remove_file(&file).ok();
}

/// S6b: a composed name carries its own layout, so a generated line
/// reads back as the same puzzle without anything being registered.
#[test]
fn k28_a_composed_tag_survives_the_round_trip() {
    round_trip(&COMPOSED_CHEAP);
}

/// The same check on the sizes actually adopted, which are minutes each
/// in a debug build.
#[test]
#[ignore = "16x16 and 20x20 are minutes each in debug; CI runs this in release"]
fn k28_the_adopted_composed_types_survive_the_round_trip() {
    round_trip(&COMPOSED);
}

fn round_trip(names: &[(&str, usize)]) {
    for &(name, size) in names {
        assert!(
            bpt_core::region::PuzzleKind::from_tag(name).is_none(),
            "{name} is a published type after all"
        );
        let out = std::env::temp_dir().join(format!("bpt-composed-{name}.txt"));
        let forged = bpt()
            .args(["forge", "--kind", name])
            .args(["--count", "2", "--seed", "5", "--level", "L2", "--out"])
            .arg(&out)
            .output()
            .expect("bpt runs");
        assert_eq!(
            forged.status.code(),
            Some(0),
            "{name}: {}",
            String::from_utf8_lossy(&forged.stderr)
        );

        let text = fs::read_to_string(&out).expect("the generated file");
        for line in text.lines() {
            assert!(line.starts_with(&format!("{name}:")), "{name}: {line}");
            assert_eq!(line.len(), name.len() + 1 + size * size);
        }

        // Solved with no geometry passed alongside: everything the
        // reader needs is in the name.
        let solved = bpt()
            .args(["solve", "--file"])
            .arg(&out)
            .arg("--unique")
            .output()
            .expect("bpt runs");
        let stdout = String::from_utf8_lossy(&solved.stdout);
        assert_eq!(solved.status.code(), Some(0), "{name}: {stdout}");
        assert!(!stdout.contains("multiple"), "{name}: {stdout}");
        assert!(
            stdout.lines().all(|l| l.starts_with(&format!("{name}:"))),
            "the answer keeps the tag: {stdout}"
        );
        fs::remove_file(&out).ok();
    }
}

/// The positional family stays on --geometry, which is the line the
/// grammar deliberately draws: where two overlapping blocks sit is a
/// choice, and a name cannot imply it.
#[test]
#[ignore = "generates a 12x12 repeatedly; CI runs this in release"]
fn k28_the_overlapping_type_works_through_its_geometry_file() {
    let out = std::env::temp_dir().join("bpt-overlap.txt");
    let forged = bpt()
        .args(["forge", "--geometry"])
        .arg(geometry("overlap8in12"))
        .args(["--count", "3", "--seed", "5", "--out"])
        .arg(&out)
        .output()
        .expect("bpt runs");
    assert_eq!(forged.status.code(), Some(0));

    let with = bpt()
        .args(["solve", "--file"])
        .arg(&out)
        .arg("--geometry")
        .arg(geometry("overlap8in12"))
        .arg("--unique")
        .output()
        .expect("bpt runs");
    assert_eq!(with.status.code(), Some(0));
    assert!(!String::from_utf8_lossy(&with.stdout).contains("multiple"));

    // And the overlap really does the work: without it, ambiguous.
    let without = bpt()
        .args(["solve", "--file"])
        .arg(&out)
        .arg("--unique")
        .output()
        .expect("bpt runs");
    assert!(
        String::from_utf8_lossy(&without.stdout).contains("multiple"),
        "the overlapping regions must be what pins the puzzle down"
    );
    fs::remove_file(&out).ok();
}
