//! The render model (AR9): a recorded event log replayed step by step.
//! Pure state — no terminal, no timers — so it is fully unit-testable.

use binsolve_core::event::SolveEvent;
use binsolve_core::grid::{Cell, Grid};
use binsolve_core::region::Puzzle;
use binsolve_core::search::{Difficulty, SolveStats, grade};
use std::time::Duration;

/// One puzzle's recorded solve, replayable to any step.
pub struct Replay {
    pub name: String,
    pub givens: Grid,
    events: Vec<SolveEvent>,
    /// Grid state after each step; index 0 is the givens.
    frames: Vec<Grid>,
    pub cursor: usize,
    pub stats: SolveStats,
    pub elapsed: Duration,
    pub solved: bool,
}

impl Replay {
    pub fn new(
        name: String,
        puzzle: &Puzzle,
        events: Vec<SolveEvent>,
        stats: SolveStats,
        elapsed: Duration,
        solved: bool,
    ) -> Self {
        let mut frames = Vec::with_capacity(events.len() + 1);
        let mut grid = puzzle.givens.clone();
        frames.push(grid.clone());
        // A refuted branch also deduced cells before it failed, so
        // undoing a guess means restoring the whole grid as it stood
        // before that guess — clearing only the guessed cell would
        // leave the branch's deductions on screen as state the solver
        // never held.
        let mut before_guess: Vec<Grid> = Vec::new();
        for event in &events {
            match event {
                SolveEvent::Deduced {
                    row, col, value, ..
                } => grid.set(*row, *col, *value),
                SolveEvent::Guessed {
                    row, col, value, ..
                } => {
                    before_guess.push(grid.clone());
                    grid.set(*row, *col, *value);
                }
                SolveEvent::Backtracked { row, col, .. } => match before_guess.pop() {
                    Some(restored) => grid = restored,
                    None => grid.set(*row, *col, Cell::Empty),
                },
                SolveEvent::SolutionFound => {}
            }
            frames.push(grid.clone());
        }
        Replay {
            name,
            givens: puzzle.givens.clone(),
            events,
            frames,
            cursor: 0,
            stats,
            elapsed,
            solved,
        }
    }

    pub fn steps(&self) -> usize {
        self.events.len()
    }

    pub fn is_finished(&self) -> bool {
        self.cursor >= self.events.len()
    }

    /// Grid as of the current step.
    pub fn grid(&self) -> &Grid {
        &self.frames[self.cursor]
    }

    /// The event that produced the current state, if any.
    pub fn current_event(&self) -> Option<&SolveEvent> {
        self.cursor.checked_sub(1).and_then(|i| self.events.get(i))
    }

    /// The cell the current step touched, for highlighting.
    pub fn current_cell(&self) -> Option<(usize, usize)> {
        match self.current_event()? {
            SolveEvent::Deduced { row, col, .. }
            | SolveEvent::Guessed { row, col, .. }
            | SolveEvent::Backtracked { row, col, .. } => Some((*row, *col)),
            SolveEvent::SolutionFound => None,
        }
    }

    pub fn step_forward(&mut self) -> bool {
        if self.is_finished() {
            return false;
        }
        self.cursor += 1;
        true
    }

    pub fn step_back(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        true
    }

    pub fn jump_to_start(&mut self) {
        self.cursor = 0;
    }

    pub fn jump_to_end(&mut self) {
        self.cursor = self.events.len();
    }

    /// Deductions, guesses and backtracks up to the current step —
    /// what the viewer has actually seen so far.
    pub fn stats_so_far(&self) -> SolveStats {
        let mut stats = SolveStats::default();
        for event in &self.events[..self.cursor] {
            match event {
                SolveEvent::Deduced { strategy, .. } => {
                    stats.deductions += 1;
                    stats.max_tier = stats.max_tier.max(strategy.tier());
                }
                SolveEvent::Guessed { .. } => stats.guesses += 1,
                SolveEvent::Backtracked { .. } => stats.backtracks += 1,
                SolveEvent::SolutionFound => {}
            }
        }
        stats
    }

    /// Final grade of the whole solve (AR8: measured pre-first-solution).
    pub fn difficulty(&self) -> Difficulty {
        grade(&self.stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use binsolve_core::event::{EventLog, SolveEvent};
    use binsolve_core::parse::parse_line;
    use binsolve_core::search::{SolveMode, SolveOutcome, solve};

    fn replay_of(line: &str) -> Replay {
        let puzzle = parse_line(line).unwrap();
        let mut log = EventLog::default();
        let outcome = solve(&puzzle, SolveMode::FirstSolution, &mut log);
        let (stats, solved) = match &outcome {
            SolveOutcome::Solved { stats, .. } => (*stats, true),
            _ => (SolveStats::default(), false),
        };
        Replay::new(
            "test".into(),
            &puzzle,
            log.events,
            stats,
            Duration::from_micros(1),
            solved,
        )
    }

    const EASY6: &str = "1..0....00.1.00..1......00.1...1..00";
    const EASY6_SOLUTION: &str = "101010010011100101011010001101110100";

    #[test]
    fn k15_replay_starts_at_the_givens_and_ends_at_the_solution() {
        let mut replay = replay_of(EASY6);
        assert_eq!(replay.grid().to_line(), EASY6);
        replay.jump_to_end();
        assert_eq!(replay.grid().to_line(), EASY6_SOLUTION);
        assert!(replay.is_finished());
        assert!(replay.solved);
    }

    #[test]
    fn k15_stepping_is_reversible() {
        let mut replay = replay_of(EASY6);
        let start = replay.grid().clone();
        for _ in 0..replay.steps() {
            replay.step_forward();
        }
        let end = replay.grid().clone();
        for _ in 0..replay.steps() {
            replay.step_back();
        }
        assert_eq!(*replay.grid(), start, "stepping back must restore givens");
        replay.jump_to_end();
        assert_eq!(*replay.grid(), end, "jumping forward must restore the end");
    }

    #[test]
    fn k15_each_step_changes_exactly_the_reported_cell() {
        let mut replay = replay_of(EASY6);
        while replay.step_forward() {
            let Some((row, col)) = replay.current_cell() else {
                continue;
            };
            let before = &replay.frames[replay.cursor - 1];
            let after = &replay.frames[replay.cursor];
            for r in 0..after.size() {
                for c in 0..after.size() {
                    if (r, c) == (row, col) {
                        continue;
                    }
                    assert_eq!(
                        before.get(r, c),
                        after.get(r, c),
                        "step {} changed r{r}c{c} besides r{row}c{col}",
                        replay.cursor
                    );
                }
            }
        }
    }

    #[test]
    fn k15_stats_accumulate_towards_the_final_total() {
        let mut replay = replay_of(EASY6);
        assert_eq!(replay.stats_so_far().deductions, 0);
        replay.jump_to_end();
        assert_eq!(replay.stats_so_far().deductions, replay.stats.deductions);
        assert_eq!(replay.difficulty().name(), "easy");
    }

    /// Regression (phase 7 audit, 2026-08-28): a Backtracked event
    /// cleared only the guessed cell, but the refuted branch had also
    /// emitted Deduced events. Those cells stayed filled, so the viewer
    /// saw a grid the solver never held — values later flipped with no
    /// event explaining them, and the displayed grid could break the
    /// rules. Undoing a guess must restore the frame from just before it.
    #[test]
    fn k15_backtracking_restores_the_frame_before_the_guess() {
        let mut replay = replay_of(&".".repeat(36));
        let mut before_guess: Vec<Grid> = Vec::new();
        let mut checked = 0usize;
        while replay.step_forward() {
            match replay.current_event() {
                Some(SolveEvent::Guessed { .. }) => {
                    // Frame before this guess = the previous frame.
                    before_guess.push(replay.frames[replay.cursor - 1].clone());
                }
                Some(SolveEvent::Backtracked { .. }) => {
                    let expected = before_guess.pop().expect("a backtrack undoes a guess");
                    assert_eq!(
                        *replay.grid(),
                        expected,
                        "after backtrack at step {} the frame must equal the one before its guess",
                        replay.cursor
                    );
                    checked += 1;
                }
                _ => {}
            }
        }
        assert!(checked > 0, "the empty 6x6 must require guessing");
    }

    #[test]
    fn k15_backtracking_replays_as_cell_removal() {
        // A grid that needs guessing: the empty 6x6.
        let mut replay = replay_of(&".".repeat(36));
        let mut saw_removal = false;
        while replay.step_forward() {
            if matches!(
                replay.current_event(),
                Some(binsolve_core::event::SolveEvent::Backtracked { .. })
            ) {
                let (row, col) = replay.current_cell().expect("backtrack names a cell");
                assert_eq!(
                    replay.grid().get(row, col),
                    Cell::Empty,
                    "backtracked cell must become empty again"
                );
                saw_removal = true;
            }
        }
        assert!(saw_removal, "empty grid should require at least one guess");
    }
}
