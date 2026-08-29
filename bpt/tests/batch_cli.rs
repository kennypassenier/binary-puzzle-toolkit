//! End-to-end tests for `bpt forge --out-dir`: the batch layout (K27),
//! the manifest (M23), all-or-nothing and directory ownership (AR29),
//! naming and exit codes (AR30), duplicate refusal (M21).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bpt() -> Command {
    Command::new(env!("CARGO_BIN_EXE_bpt"))
}

/// Each test owns a directory named after itself, so a failure leaves
/// the evidence behind instead of a shared scratch area.
fn workdir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bpt-batch-{name}"));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn forge(dir: &Path, extra: &[&str]) -> Output {
    let mut cmd = bpt();
    cmd.args(["forge", "--out-dir"])
        .arg(dir)
        .args(["--kind", "6", "--seed", "4242"])
        .args(extra);
    cmd.output().expect("bpt runs")
}

fn manifest(dir: &Path) -> bpt_forge::manifest::Manifest {
    let text = fs::read_to_string(dir.join("manifest.json")).expect("manifest exists");
    bpt_forge::manifest::Manifest::from_json(&text).expect("manifest parses")
}

#[test]
fn k27_a_batch_of_100_lands_in_the_expected_structure() {
    let dir = workdir("hundred");
    let out = forge(&dir, &["--count", "100"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let manifest = manifest(&dir);
    assert_eq!(manifest.requested, 100);
    assert_eq!(manifest.completed, 100);
    assert_eq!(manifest.puzzles.len(), 100);

    // Every manifest entry names a file that is really there, in the
    // two-line corpus shape, and the digest still matches its contents.
    for entry in &manifest.puzzles {
        let body = fs::read_to_string(dir.join(&entry.file))
            .unwrap_or_else(|_| panic!("{} is missing", entry.file));
        let mut lines = body.lines();
        let puzzle = lines.next().expect("a puzzle line");
        let solution = lines.next().expect("a solution line");
        assert!(
            solution.starts_with("solution:"),
            "{} : {solution}",
            entry.file
        );
        assert_eq!(entry.digest, bpt_forge::manifest::digest(puzzle));
        assert!(
            entry.file.starts_with("bf-4242-") && entry.file.ends_with(".txt"),
            "AR30 names a file after what reproduces it: {}",
            entry.file
        );
        assert!(entry.file.contains(entry.level.name()), "{}", entry.file);
    }

    // And nothing else: no torn temp file survived the run.
    let strays: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".tmp"))
        .collect();
    assert!(strays.is_empty(), "left behind: {strays:?}");

    let flat = fs::read_to_string(dir.join("puzzles.txt")).expect("flat file exists");
    assert_eq!(flat.lines().count(), 100);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn ar27_the_flat_file_is_what_the_solver_can_prove_unique() {
    let dir = workdir("provable");
    assert_eq!(forge(&dir, &["--count", "8"]).status.code(), Some(0));

    let out = bpt()
        .args(["solve", "--file"])
        .arg(dir.join("puzzles.txt"))
        .arg("--unique")
        .output()
        .expect("bpt runs");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.lines().count(), 8);
    assert!(
        !stdout.contains("multiple") && !stdout.contains("unsolvable"),
        "every generated puzzle must have exactly one solution:\n{stdout}"
    );

    // The two-line files must satisfy --check, which validates the
    // supplied solution rather than proving uniqueness (AR27).
    let first = manifest(&dir).puzzles[0].file.clone();
    let checked = bpt()
        .args(["solve", "--check", "--file"])
        .arg(dir.join(&first))
        .output()
        .expect("bpt runs");
    assert_eq!(checked.status.code(), Some(0), "{first}");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn ar29_a_batch_owns_its_directory() {
    let dir = workdir("owned");
    assert_eq!(forge(&dir, &["--count", "3"]).status.code(), Some(0));

    let refused = forge(&dir, &["--count", "3"]);
    assert_eq!(refused.status.code(), Some(2), "a second run must refuse");
    let message = String::from_utf8_lossy(&refused.stderr);
    assert!(
        message.contains("--force"),
        "and say how to proceed: {message}"
    );
    // The refusal changed nothing.
    assert_eq!(manifest(&dir).completed, 3);

    let forced = forge(&dir, &["--count", "3", "--force"]);
    assert_eq!(
        forced.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&forced.stderr)
    );
    let flat = fs::read_to_string(dir.join("puzzles.txt")).unwrap();
    let distinct: std::collections::HashSet<_> = flat.lines().collect();
    assert_eq!(distinct.len(), flat.lines().count(), "M21: no duplicates");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn ar30_a_short_batch_is_partial_not_a_failure() {
    let dir = workdir("exhausted");
    // 4x4 is the one geometry small enough to run out of distinct
    // puzzles: past roughly 3990 every attempt reproduces one already
    // generated, so the batch comes up short without anything failing.
    let out = bpt()
        .args(["forge", "--out-dir"])
        .arg(&dir)
        .args(["--kind", "4", "--count", "4000", "--seed", "3"])
        .output()
        .expect("bpt runs");
    assert_eq!(
        out.status.code(),
        Some(1),
        "AR30: short of what was asked is exit 1, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let manifest = manifest(&dir);
    assert_eq!(manifest.status, bpt_forge::manifest::Status::Partial);
    assert!(manifest.completed < manifest.requested);
    assert_eq!(manifest.completed as usize, manifest.puzzles.len());
    assert!(!manifest.notes.is_empty(), "a partial batch must say why");
    assert!(
        manifest.notes[0].contains("already generated"),
        "and say which why: {}",
        manifest.notes[0]
    );
    // Everything it did produce is still on disk and still distinct.
    let flat = fs::read_to_string(dir.join("puzzles.txt")).unwrap();
    let distinct: std::collections::HashSet<_> = flat.lines().collect();
    assert_eq!(distinct.len(), manifest.completed as usize);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn ar30_an_impossible_request_is_a_usage_error() {
    let dir = workdir("bad-level");
    let out = bpt()
        .args(["forge", "--out-dir"])
        .arg(&dir)
        .args(["--kind", "6", "--level", "L9"])
        .output()
        .expect("bpt runs");
    assert_eq!(out.status.code(), Some(2));
    assert!(
        !dir.exists(),
        "a rejected run must not create its directory"
    );
}

#[test]
fn m20_the_same_seed_reproduces_the_same_batch() {
    let (a, b) = (workdir("repro-a"), workdir("repro-b"));
    assert_eq!(forge(&a, &["--count", "6"]).status.code(), Some(0));
    assert_eq!(forge(&b, &["--count", "6"]).status.code(), Some(0));
    assert_eq!(
        fs::read_to_string(a.join("puzzles.txt")).unwrap(),
        fs::read_to_string(b.join("puzzles.txt")).unwrap()
    );
    // The manifests differ only in timing, which is why elapsed_ms is
    // the one field a reproducibility check must ignore.
    let (ma, mb) = (manifest(&a), manifest(&b));
    assert_eq!(
        ma.puzzles.iter().map(|p| &p.digest).collect::<Vec<_>>(),
        mb.puzzles.iter().map(|p| &p.digest).collect::<Vec<_>>()
    );
    fs::remove_dir_all(&a).ok();
    fs::remove_dir_all(&b).ok();
}

#[test]
fn m26_a_piped_batch_carries_no_progress_noise() {
    let dir = workdir("piped");
    let out = forge(&dir, &["--count", "5"]);
    assert_eq!(out.status.code(), Some(0));
    // Command::output() gives the child pipes, not a terminal, so this
    // is exactly the case M26 promises stays clean.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.is_empty(), "a pipe must get nothing: {stderr:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("forging") && !stdout.contains('\r'),
        "and the data stream must stay data: {stdout:?}"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn ar29_a_failed_batch_leaves_nothing_behind() {
    let dir = workdir("discarded");
    // Learn the exact filenames this seed produces, then sabotage one of
    // them: a directory where a file must go makes the rename fail for
    // reasons no retry can fix.
    let scout = workdir("discarded-scout");
    assert_eq!(forge(&scout, &["--count", "5"]).status.code(), Some(0));
    let victim = manifest(&scout).puzzles[3].file.clone();
    fs::remove_dir_all(&scout).ok();

    fs::create_dir_all(dir.join(&victim)).expect("sabotage in place");
    let out = forge(&dir, &["--count", "5", "--force"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "a batch that cannot be written must fail loudly"
    );

    let left: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| *name != victim)
        .collect();
    assert!(
        left.is_empty(),
        "AR29: an error discards the batch, but these survived: {left:?}"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn m22_a_parallel_batch_equals_a_sequential_one() {
    // The CLI generates on all cores; the library's own sequential path
    // is the reference. Same seed must mean the same puzzles, in the
    // same order, whichever produced them.
    let dir = workdir("parallel");
    assert_eq!(forge(&dir, &["--count", "24"]).status.code(), Some(0));

    let plan = bpt_forge::batch::Plan::new(
        6,
        vec![bpt_core::region::Region::square(0, 0, 6)],
        bpt_forge::grade::Level::L4,
        4242,
        24,
    );
    let reference = bpt_forge::batch::run(&plan, &mut std::collections::HashSet::new());
    let sequential: Vec<String> = reference
        .produced
        .iter()
        .map(|p| format!("{}\n", p.carved.puzzle.to_line()))
        .collect();
    assert_eq!(
        fs::read_to_string(dir.join("puzzles.txt")).unwrap(),
        sequential.concat()
    );

    // The recorded triples must match too, or a manifest from a parallel
    // run would not restore under a sequential one.
    let manifest = manifest(&dir);
    for (entry, produced) in manifest.puzzles.iter().zip(&reference.produced) {
        assert_eq!(entry.index, produced.index);
        assert_eq!(entry.attempt, produced.attempt);
    }
    fs::remove_dir_all(&dir).ok();
}

/// AR29b: Ctrl-C writes what is finished, with a status no consumer can
/// mistake for a complete batch. Unix-only because the test has to send
/// the signal itself.
#[cfg(unix)]
#[test]
fn m26_a_cancelled_batch_is_valid_and_says_so() {
    use std::process::Stdio;

    let dir = workdir("cancelled");
    // A cheap geometry with a long queue: the run is still going when
    // the signal arrives, and the chunk in flight finishes quickly, so
    // the test costs seconds rather than minutes. A large grid would
    // test the same thing and make the suite unusable.
    let child = bpt()
        .args(["forge", "--out-dir"])
        .arg(&dir)
        .args(["--kind", "10", "--count", "3000", "--seed", "8"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("bpt starts");

    std::thread::sleep(std::time::Duration::from_secs(2));
    let killed = std::process::Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .expect("kill runs");
    assert!(killed.success(), "could not signal the run");

    let out = child.wait_with_output().expect("bpt finishes");
    assert_eq!(
        out.status.code(),
        Some(3),
        "a cancelled batch has its own exit code: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let manifest = manifest(&dir);
    assert_eq!(manifest.status, bpt_forge::manifest::Status::Cancelled);
    assert!(
        manifest.completed < manifest.requested,
        "a cancelled batch is shorter than what was asked for"
    );
    assert_eq!(manifest.completed as usize, manifest.puzzles.len());

    // What landed is complete and valid, which is the whole promise.
    for entry in &manifest.puzzles {
        let body = fs::read_to_string(dir.join(&entry.file)).expect("a finished file");
        assert!(body.lines().count() == 2, "{} is torn", entry.file);
        assert_eq!(
            entry.digest,
            bpt_forge::manifest::digest(body.lines().next().unwrap())
        );
    }
    let validated = bpt()
        .args(["solve", "--file"])
        .arg(dir.join("puzzles.txt"))
        .arg("--unique")
        .output()
        .expect("bpt runs");
    assert_eq!(
        validated.status.code(),
        Some(0),
        "the partial batch must still validate"
    );
    fs::remove_dir_all(&dir).ok();
}
