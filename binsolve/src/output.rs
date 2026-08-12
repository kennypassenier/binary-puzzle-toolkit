//! Output shaping: canonical lines, failure markers, pretty grid and
//! atomic file writes (AR7, AR11, K10, K11, K12).

use binsolve_core::grid::Grid;
use binsolve_core::region::Puzzle;
use binsolve_core::search::{SolveOutcome, SolveStats};
use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

/// AR7 failure markers. A failed puzzle keeps its original line so the
/// batch mapping stays 1:1 and the input is recoverable.
pub fn marker_line(marker: &str, original: &str) -> String {
    format!("#{marker}:{original}")
}

/// The canonical single-line result for one puzzle (K10).
pub fn canonical_line(outcome: &SolveOutcome, puzzle: &Puzzle, original: &str) -> String {
    match outcome {
        SolveOutcome::Solved { solution, .. } => match puzzle.kind.tag() {
            Some(tag) => format!("{tag}:{}", solution.to_line()),
            None => solution.to_line(),
        },
        SolveOutcome::MultipleSolutions { .. } => marker_line("multiple", original),
        SolveOutcome::Contradiction { .. } => marker_line("contradiction", original),
        SolveOutcome::Stuck { .. } => marker_line("stuck", original),
    }
}

/// Human-readable grid with a blank column between cells (K11).
pub fn pretty_grid(grid: &Grid) -> String {
    grid.to_string()
}

/// The stats block shown on a terminal (K11).
pub fn pretty_stats(stats: &SolveStats, elapsed: Duration, unique: Option<bool>) -> String {
    let mut out = format!(
        "solved in {:.1?} — {} deductions, {} guesses, {} backtracks",
        elapsed, stats.deductions, stats.guesses, stats.backtracks
    );
    match unique {
        Some(true) => out.push_str(", solution proven unique"),
        Some(false) => out.push_str(", multiple solutions exist"),
        None => {}
    }
    out
}

/// Atomically replace `path` with `content` (AR11): temp file in the
/// destination directory, flush + sync, rename over the target. On
/// Windows a sharing violation (virus scanner, open editor) is retried
/// briefly before failing.
pub fn write_atomic(path: &Path, content: &str) -> io::Result<()> {
    use std::io::Write as _;

    let dir = path.parent().filter(|p| !p.as_os_str().is_empty());
    let temp = match dir {
        Some(dir) => dir.join(format!(
            ".{}.tmp",
            path.file_name().unwrap_or_default().to_string_lossy()
        )),
        None => Path::new(&format!(
            ".{}.tmp",
            path.file_name().unwrap_or_default().to_string_lossy()
        ))
        .to_path_buf(),
    };
    {
        let mut file = fs::File::create(&temp)?;
        file.write_all(content.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
    }
    let mut last_err = None;
    for attempt in 0..5 {
        match fs::rename(&temp, path) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                std::thread::sleep(Duration::from_millis(20 * (attempt + 1)));
            }
        }
    }
    let _ = fs::remove_file(&temp);
    Err(last_err.unwrap_or_else(|| io::Error::other("rename failed")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use binsolve_core::parse::parse_line;

    #[test]
    fn k12_markers_preserve_the_original_line() {
        assert_eq!(marker_line("stuck", "10.."), "#stuck:10..");
    }

    #[test]
    fn k10_solved_line_keeps_the_tag() {
        let puzzle = parse_line(&format!("8in14:{}", ".".repeat(196))).unwrap();
        let grid = Grid::empty(14);
        let outcome = SolveOutcome::Solved {
            solution: grid,
            stats: SolveStats::default(),
        };
        let line = canonical_line(&outcome, &puzzle, "irrelevant");
        assert!(line.starts_with("8in14:"));
    }

    #[test]
    fn ar11_atomic_write_replaces_existing_file() {
        let dir = std::env::temp_dir().join("binsolve-atomic-test");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("atomic-test.txt");
        write_atomic(&path, "first\n").unwrap();
        write_atomic(&path, "second\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "second\n");
        // No temp files left behind.
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "temp files left: {leftovers:?}");
        fs::remove_dir_all(&dir).unwrap();
    }
}
