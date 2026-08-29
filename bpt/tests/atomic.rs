//! AR8/AR10 and standing rules 12 and 15: writes land whole or not at
//! all, and an interrupted run leaves recoverable state. Real files,
//! real renames — nothing mocked (standing rule 9).

use std::path::PathBuf;

/// Each test gets its own directory so a failure leaves evidence behind
/// without disturbing the others.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("binforge-atomic-{}-{name}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// The module under test lives in the binary crate, so the test includes
// it directly: it has no dependencies of its own beyond std and anyhow.
#[path = "../src/atomic.rs"]
mod atomic;

#[test]
fn ar8_write_creates_the_file_with_exact_contents() {
    let dir = scratch("create");
    let path = dir.join("bf-42-0-L2.txt");
    atomic::write(&path, "4x8x8:110..0.1\n").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "4x8x8:110..0.1\n");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn ar8_overwrite_replaces_the_old_content_completely() {
    let dir = scratch("overwrite");
    let path = dir.join("manifest.json");
    atomic::write(&path, "{\"completed\": 1}").unwrap();
    atomic::write(&path, "{\"completed\": 100}").unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text, "{\"completed\": 100}");
    // No remnant of the longer-or-shorter previous write.
    assert!(!text.contains("\"completed\": 1}"));
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn ar8_no_temp_file_survives_a_successful_write() {
    let dir = scratch("no-temp");
    atomic::write(&dir.join("a.txt"), "x").unwrap();
    let leftovers: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "tmp"))
        .collect();
    assert!(leftovers.is_empty(), "temp file left behind: {leftovers:?}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn ar10_orphans_from_an_interrupted_run_are_cleaned_and_counted() {
    // Simulates the power-loss trace: temp files written, renames never
    // reached. The next run must clear them and say how many.
    let dir = scratch("orphans");
    std::fs::write(dir.join("bf-1-0-L2.txt.tmp"), "half a puzzle").unwrap();
    std::fs::write(dir.join("bf-1-1-L2.txt.tmp"), "half a puzzle").unwrap();
    std::fs::write(dir.join("bf-1-2-L2.txt"), "a complete puzzle").unwrap();

    let removed = atomic::clean_orphans(&dir).unwrap();
    assert_eq!(removed, 2, "both temp files are removed");
    // The complete file from before the crash is untouched: it was
    // renamed into place, so it is whole by construction.
    assert_eq!(
        std::fs::read_to_string(dir.join("bf-1-2-L2.txt")).unwrap(),
        "a complete puzzle"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn ar10_cleaning_a_missing_directory_is_not_an_error() {
    // A first run has nothing to clean; that is not a failure.
    let missing = std::env::temp_dir().join("binforge-atomic-does-not-exist");
    std::fs::remove_dir_all(&missing).ok();
    assert_eq!(atomic::clean_orphans(&missing).unwrap(), 0);
}

#[test]
fn ar8_sync_dir_succeeds_on_a_real_directory() {
    let dir = scratch("sync");
    atomic::write(&dir.join("a.txt"), "x").unwrap();
    atomic::sync_dir(&dir).unwrap();
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn ar8_write_creates_missing_directories() {
    let dir = scratch("nested");
    let path = dir.join("standard").join("10").join("bf-7-0-L1.txt");
    atomic::write(&path, "..1.\n").unwrap();
    assert!(path.exists());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn ar10_a_permanent_rename_failure_is_reported_without_five_retries() {
    // The destination already exists as a directory: renaming onto it can
    // never succeed, so retrying only delays the answer and blames a file
    // lock that is not there.
    let dir = scratch("permanent");
    let blocked = dir.join("occupied");
    std::fs::create_dir_all(blocked.join("child")).unwrap();

    let started = std::time::Instant::now();
    let error = atomic::write(&blocked, "x").unwrap_err();
    let elapsed = started.elapsed();

    let text = format!("{error:#}");
    assert!(text.contains("Remedy:"), "{text}");
    assert!(
        text.contains("a directory already exists"),
        "must name the real cause, not a file lock: {text}"
    );
    // Five backoffs take at least 200 ms; failing fast is the point. This
    // is the assertion that caught the real bug: on Windows, renaming onto
    // a directory reports PermissionDenied, which an ErrorKind-based
    // retry rule mistook for a transient file lock.
    assert!(elapsed.as_millis() < 150, "took {elapsed:?}, expected fast");

    std::fs::remove_dir_all(&dir).ok();
}
