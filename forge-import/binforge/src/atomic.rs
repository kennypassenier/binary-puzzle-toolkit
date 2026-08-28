//! AR8/AR10: every file lands complete or not at all, and a batch commits
//! completely or not at all. Standing rule 12 (atomic writes) and rule 15
//! (reason through power loss) are the whole point of this module.
//!
//! Power-loss behaviour, deliberately: a temp file in the *destination*
//! directory is written, flushed and synced, then renamed over the target.
//! A crash therefore leaves either the old complete file or the new
//! complete file, plus at most an orphan `.tmp` that the next run removes.
//! The rename only becomes durable once the *directory* is synced, so a
//! batch syncs its directory before writing the manifest and again after —
//! without that, a crash could surface a manifest claiming files that are
//! not there.
//!
//! Honest limitation: that directory sync is a Unix facility. On Windows
//! std cannot open a directory to flush it, so the ordering guarantee
//! there rests on the filesystem alone and is weaker than on Linux. C3
//! makes Windows a supported platform, so this is recorded as a known
//! limitation for the Phase 7 test plan rather than claimed as solved.

// Built in L1 while the solver-dependent milestones were blocked, and
// wired into the CLI in L5 where batches are actually written. Marked
// rather than silently tolerated: if L5 lands and this is still unused,
// something went wrong.
#![allow(dead_code)]

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};

/// Windows keeps a handle briefly after a process exits (indexers, virus
/// scanners), so a rename can fail with a sharing violation that is gone
/// milliseconds later. Bounded — never an unbounded spin (AR10).
const RENAME_ATTEMPTS: u32 = 5;
const RENAME_BACKOFF: Duration = Duration::from_millis(20);

/// Write `contents` to `path`, atomically. The temp file is created in the
/// same directory so the rename cannot cross a filesystem boundary.
pub fn write(path: &Path, contents: &str) -> Result<()> {
    // Checked up front rather than inferred from the rename's error code:
    // Windows CI showed it reports this permanent situation with a code
    // that also means "file temporarily locked", so classifying after the
    // fact retried a doomed rename five times and then blamed the wrong
    // cause. The condition itself is unambiguous on every platform.
    if path.is_dir() {
        bail!(
            "cannot write {} — a directory already exists at that path\n\
             Remedy: pick a different output path, or remove the directory; \
             a puzzle file and a directory cannot share a name.",
            path.display()
        );
    }

    let dir = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;

    let temp = temp_path(path);
    {
        let mut file = File::create(&temp)
            .with_context(|| format!("cannot create temp file {}", temp.display()))?;
        file.write_all(contents.as_bytes())
            .with_context(|| format!("cannot write {}", temp.display()))?;
        file.sync_all()
            .with_context(|| format!("cannot flush {} to disk", temp.display()))?;
    }

    rename_with_retry(&temp, path)?;
    Ok(())
}

/// Make the renames in `dir` durable. Called after the last file of a
/// batch and again after the manifest, so "the manifest exists" really
/// does imply "its files exist" (AR10). Effective on Unix; see the module
/// note for why Windows gets a weaker guarantee.
pub fn sync_dir(dir: &Path) -> Result<()> {
    // Only Unix can open a directory as a file; on Windows the rename is
    // already ordered by the filesystem, so this is a no-op there.
    #[cfg(unix)]
    {
        let handle =
            File::open(dir).with_context(|| format!("cannot open {} to sync", dir.display()))?;
        handle
            .sync_all()
            .with_context(|| format!("cannot sync directory {}", dir.display()))?;
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
    }
    Ok(())
}

/// Remove `.tmp` leftovers from an interrupted earlier run. Reported by
/// count rather than silently, so a repeatedly crashing run is visible.
///
/// **Call this only before any writer starts.** It cannot tell an orphan
/// from a temp file another worker is writing right now, so calling it
/// while a parallel batch (M3) is running would delete work in flight and
/// surface as a rename failure.
pub fn clean_orphans(dir: &Path) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }
    let mut removed = 0;
    for entry in fs::read_dir(dir).with_context(|| format!("cannot read {}", dir.display()))? {
        let entry = entry.with_context(|| format!("cannot read an entry of {}", dir.display()))?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "tmp") {
            fs::remove_file(&path)
                .with_context(|| format!("cannot remove orphan {}", path.display()))?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

/// Only ERROR_SHARING_VIOLATION is worth retrying: it means another
/// process holds the file open right now. Windows CI proved that
/// `ErrorKind::PermissionDenied` is too coarse — renaming onto an
/// existing directory reports the same kind, so a permanent failure was
/// retried five times and then blamed on a lock that never existed.
/// Unix has no transient rename failure of this sort at all.
fn is_transient(error: &std::io::Error) -> bool {
    #[cfg(windows)]
    {
        const ERROR_SHARING_VIOLATION: i32 = 32;
        error.raw_os_error() == Some(ERROR_SHARING_VIOLATION)
    }
    #[cfg(not(windows))]
    {
        let _ = error;
        false
    }
}

fn rename_with_retry(from: &Path, to: &Path) -> Result<()> {
    let mut last_error = None;
    for attempt in 0..RENAME_ATTEMPTS {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(error) => {
                let transient = is_transient(&error);
                last_error = Some(error);
                if !transient {
                    break;
                }
                if attempt + 1 < RENAME_ATTEMPTS {
                    std::thread::sleep(RENAME_BACKOFF * (attempt + 1));
                }
            }
        }
    }
    let error = last_error.expect("at least one attempt was made");
    Err(anyhow::Error::new(error).context(format!(
        "cannot move {} into place as {}\n\
         Remedy: if the destination is locked, another program may be holding \
         it open (on Windows an indexer or virus scanner) — close it and run \
         again; otherwise check that the path is writable and is not a \
         directory. The temp file is left behind and the next run cleans it up.",
        from.display(),
        to.display()
    )))
}
