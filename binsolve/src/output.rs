//! Output shaping: canonical lines, failure markers, pretty grid and
//! atomic file writes (AR7, AR11, K10, K11, K12).

use binsolve_core::grid::Grid;
use binsolve_core::region::Puzzle;
use binsolve_core::search::{SolveOutcome, SolveStats, grade};
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

/// The stats block shown on a terminal (K11, M2).
pub fn pretty_stats(stats: &SolveStats, elapsed: Duration, unique: Option<bool>) -> String {
    let mut out = format!(
        "solved in {:.1?} — {} deductions, {} guesses, {} backtracks, graded {}",
        elapsed,
        stats.deductions,
        stats.guesses,
        stats.backtracks,
        grade(stats).name()
    );
    match unique {
        Some(true) => out.push_str(", solution proven unique"),
        Some(false) => out.push_str(", multiple solutions exist"),
        None => {}
    }
    out
}

/// The terminal-only display for one puzzle (K11): the grid plus a
/// summary line. Returns None when nothing should be shown.
///
/// `interactive` is a parameter rather than a call to `is_terminal()`
/// inside this function, so every branch is reachable from a test. That
/// matters more than it looks: if this decision were made here, nothing
/// could catch the grid leaking into a pipe and breaking the contract
/// that pipes receive canonical lines only (found by the phase 7 audit,
/// which noted this surface had no test at all).
pub fn terminal_display(
    outcome: &SolveOutcome,
    elapsed: Duration,
    prove_unique: bool,
    interactive: bool,
) -> Option<String> {
    if !interactive {
        return None;
    }
    let mut out = String::new();
    match outcome {
        SolveOutcome::Solved { solution, stats } => {
            out.push_str(&pretty_grid(solution));
            out.push_str(&pretty_stats(
                stats,
                elapsed,
                if prove_unique { Some(true) } else { None },
            ));
            out.push('\n');
        }
        SolveOutcome::MultipleSolutions { first, .. } => {
            out.push_str(&pretty_grid(first));
            out.push_str("multiple solutions exist — this puzzle is not unique\n");
        }
        SolveOutcome::Contradiction { reason } => {
            out.push_str(&format!("no solution: {reason}\n"));
        }
        SolveOutcome::Stuck { grid, filled } => {
            out.push_str(&pretty_grid(grid));
            let total = grid.size() * grid.size();
            out.push_str(&format!(
                "strategies alone reached {filled}/{total} cells ({}%)\n",
                filled * 100 / total
            ));
        }
    }
    Some(out)
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
            Ok(()) => {
                remove_orphan_temps(path);
                return Ok(());
            }
            Err(e) => {
                last_err = Some(e);
                std::thread::sleep(Duration::from_millis(20 * (attempt + 1)));
            }
        }
    }
    let _ = fs::remove_file(&temp);
    Err(last_err.unwrap_or_else(|| io::Error::other("rename failed")))
}

/// AR11 promises "at most an orphan `.tmp` cleaned by the next run".
/// A crash or power cut between creating the temp file and renaming it
/// leaves one behind; nothing else ever writes this pattern, so a
/// successful write is the right moment to sweep them.
fn remove_orphan_temps(path: &Path) {
    let Some(dir) = path.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return;
    };
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return;
    };
    let current = format!(".{name}.tmp");
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Some(found) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        // Only our own leftovers: the exact name this destination uses.
        if found == current {
            let _ = fs::remove_file(entry.path());
        }
    }
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

    fn solved_outcome() -> SolveOutcome {
        let puzzle = parse_line("1..0....00.1.00..1......00.1...1..00").unwrap();
        binsolve_core::search::solve(
            &puzzle,
            binsolve_core::search::SolveMode::FirstSolution,
            &mut binsolve_core::event::NullObserver,
        )
    }

    /// K11: the grid and statistics must NEVER reach a pipe — that is
    /// the contract the scraper pipeline depends on.
    #[test]
    fn k11_nothing_is_displayed_when_output_is_not_a_terminal() {
        let outcome = solved_outcome();
        assert!(
            terminal_display(&outcome, Duration::from_micros(1), false, false).is_none(),
            "a pipe must receive no grid, no statistics, nothing"
        );
    }

    #[test]
    fn k11_every_outcome_shape_renders_on_a_terminal() {
        let d = Duration::from_micros(58);

        let text = terminal_display(&solved_outcome(), d, false, true).expect("solved renders");
        assert!(text.contains("1 0 1 0 1 0"), "the grid is shown: {text}");
        assert!(text.contains("graded easy"), "the grade is shown: {text}");
        assert!(
            !text.contains("proven unique"),
            "uniqueness is only claimed when it was proven: {text}"
        );

        let text = terminal_display(&solved_outcome(), d, true, true).expect("solved renders");
        assert!(
            text.contains("solution proven unique"),
            "proving uniqueness must be stated: {text}"
        );

        let empty = parse_line(&".".repeat(16)).unwrap();
        let multiple = binsolve_core::search::solve(
            &empty,
            binsolve_core::search::SolveMode::ProveUniqueness,
            &mut binsolve_core::event::NullObserver,
        );
        let text = terminal_display(&multiple, d, true, true).expect("multiple renders");
        assert!(text.contains("not unique"), "{text}");

        let bad = parse_line(&format!("000{}", ".".repeat(33))).unwrap();
        let contradiction = binsolve_core::search::solve(
            &bad,
            binsolve_core::search::SolveMode::FirstSolution,
            &mut binsolve_core::event::NullObserver,
        );
        let text = terminal_display(&contradiction, d, false, true).expect("contradiction renders");
        assert!(text.starts_with("no solution: "), "{text}");
        assert!(text.contains("three consecutive"), "names the rule: {text}");

        let stuck = binsolve_core::search::solve(
            &empty,
            binsolve_core::search::SolveMode::StrategiesOnly,
            &mut binsolve_core::event::NullObserver,
        );
        let text = terminal_display(&stuck, d, false, true).expect("stuck renders");
        assert!(text.contains("strategies alone reached 0/16"), "{text}");
    }

    /// AR11 promised orphan cleanup and nothing implemented it (found
    /// by the phase 7 audit). Simulate the crash window by leaving a
    /// temp file behind, then write normally: it must be gone.
    #[test]
    fn ar11_a_successful_write_sweeps_an_orphaned_temp() {
        let dir = std::env::temp_dir().join("binsolve-orphan-test");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("out.txt");
        let orphan = dir.join(".out.txt.tmp");
        fs::write(&orphan, "left behind by a crash").unwrap();
        assert!(orphan.exists(), "the simulated orphan must exist first");

        write_atomic(&path, "fresh\n").unwrap();

        assert!(!orphan.exists(), "a successful write must sweep the orphan");
        assert_eq!(fs::read_to_string(&path).unwrap(), "fresh\n");
        fs::remove_dir_all(&dir).unwrap();
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
