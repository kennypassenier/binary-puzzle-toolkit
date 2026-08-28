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

// Built in L1 while the solver-dependent milestones were blocked, and
// wired into the CLI in L5 where batches are actually written. Marked
// rather than silently tolerated: if L5 lands and this is still unused,
// something went wrong.
#![allow(dead_code)]

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};

/// Windows keeps a handle briefly after a process exits (indexers, virus
/// scanners), so a rename can fail with a sharing violation that is gone
/// milliseconds later. Bounded — never an unbounded spin (AR10).
const RENAME_ATTEMPTS: u32 = 5;
const RENAME_BACKOFF: Duration = Duration::from_millis(20);

/// Write `contents` to `path`, atomically. The temp file is created in the
/// same directory so the rename cannot cross a filesystem boundary.
pub fn write(path: &Path, contents: &str) -> Result<()> {
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
/// does imply "its files exist" (AR10).
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

fn rename_with_retry(from: &Path, to: &Path) -> Result<()> {
    let mut last_error = None;
    for attempt in 0..RENAME_ATTEMPTS {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = Some(error);
                if attempt + 1 < RENAME_ATTEMPTS {
                    std::thread::sleep(RENAME_BACKOFF * (attempt + 1));
                }
            }
        }
    }
    let error = last_error.expect("at least one attempt was made");
    Err(anyhow::Error::new(error).context(format!(
        "cannot move {} into place as {} after {RENAME_ATTEMPTS} attempts\n\
         Remedy: another program may be holding the file open (on Windows an \
         indexer or virus scanner); close it and run again. The temp file is \
         left behind and will be cleaned up by the next run.",
        from.display(),
        to.display()
    )))
}
