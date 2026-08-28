//! Widget layout for the replay view. Kept separate from the event
//! loop so the render model stays testable without a terminal.

use crate::replay::Replay;
use bpt_core::event::SolveEvent;
use bpt_core::grid::Cell;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Wrap};

/// One-line description of the step currently shown.
pub fn step_description(replay: &Replay) -> String {
    match replay.current_event() {
        None => format!("{} — givens", replay.name),
        Some(SolveEvent::Deduced {
            row,
            col,
            value,
            strategy,
            ..
        }) => format!(
            "{}: {} → r{row}c{col} = {}",
            replay.cursor,
            strategy.name(),
            value.to_char()
        ),
        Some(SolveEvent::Guessed {
            row,
            col,
            value,
            depth,
        }) => format!(
            "{}: guess (depth {depth}) → r{row}c{col} = {}",
            replay.cursor,
            value.to_char()
        ),
        Some(SolveEvent::Backtracked { row, col, to_depth }) => {
            format!(
                "{}: backtrack to depth {to_depth} → r{row}c{col} cleared",
                replay.cursor
            )
        }
        Some(SolveEvent::SolutionFound) => format!("{}: solution found", replay.cursor),
    }
}

/// The stats line under the grid.
pub fn stats_line(replay: &Replay) -> String {
    let so_far = replay.stats_so_far();
    format!(
        "step {}/{}  ·  {} deductions  ·  {} guesses  ·  {} backtracks  ·  solved in {:.1?} ({})",
        replay.cursor,
        replay.steps(),
        so_far.deductions,
        so_far.guesses,
        so_far.backtracks,
        replay.elapsed,
        replay.difficulty().name()
    )
}

fn cell_style(is_given: bool, is_current: bool) -> Style {
    if is_current {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else if is_given {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    }
}

/// Render the grid as styled lines: givens bold, deduced cells cyan,
/// the cell touched by the current step highlighted.
pub fn grid_lines(replay: &Replay) -> Vec<Line<'static>> {
    let grid = replay.grid();
    let current = replay.current_cell();
    (0..grid.size())
        .map(|r| {
            let spans: Vec<Span> = (0..grid.size())
                .map(|c| {
                    let cell = grid.get(r, c);
                    let text = match cell {
                        Cell::Empty => "· ".to_string(),
                        other => format!("{} ", other.to_char()),
                    };
                    Span::styled(
                        text,
                        cell_style(!replay.givens.get(r, c).is_empty(), current == Some((r, c))),
                    )
                })
                .collect();
            Line::from(spans)
        })
        .collect()
}

pub struct ViewState<'a> {
    pub replays: &'a [Replay],
    pub selected: usize,
    pub playing: bool,
    pub speed: u64,
}

pub fn render(frame: &mut Frame, state: &ViewState<'_>) {
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(4),
    ])
    .split(frame.area());

    let replay = &state.replays[state.selected];

    let header = Paragraph::new(format!(
        "{}  ({}/{} puzzles)  ·  {}  ·  {}",
        replay.name,
        state.selected + 1,
        state.replays.len(),
        if replay.solved {
            "solved"
        } else {
            "no unique solution"
        },
        if state.playing {
            format!("playing at {} steps/s", state.speed)
        } else {
            "paused".to_string()
        }
    ))
    .block(Block::default().borders(Borders::ALL).title("binsolve"));
    frame.render_widget(header, chunks[0]);

    let body = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let grid = Paragraph::new(grid_lines(replay))
        .block(Block::default().borders(Borders::ALL).title("grid"));
    frame.render_widget(grid, body[0]);

    let trace = Paragraph::new(step_description(replay))
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("current step"));
    frame.render_widget(trace, body[1]);

    let progress = if replay.steps() == 0 {
        100
    } else {
        (replay.cursor * 100 / replay.steps()) as u16
    };
    // The gauge is bordered: it needs three rows (border, label, border).
    let footer = Layout::vertical([Constraint::Length(3), Constraint::Length(1)]).split(chunks[2]);
    frame.render_widget(
        Gauge::default()
            .block(Block::default().borders(Borders::ALL))
            .percent(progress)
            .label(stats_line(replay)),
        footer[0],
    );
    frame.render_widget(
        Paragraph::new(
            "space play/pause  ·  ←/→ step  ·  ↑/↓ puzzle  ·  +/- speed  ·  home/end  ·  q quit",
        ),
        footer[1],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::Replay;
    use bpt_core::event::EventLog;
    use bpt_core::parse::parse_line;
    use bpt_core::search::{SolveMode, SolveOutcome, SolveStats, solve};
    use std::time::Duration;

    fn sample() -> Replay {
        let puzzle = parse_line("1..0....00.1.00..1......00.1...1..00").unwrap();
        let mut log = EventLog::default();
        let outcome = solve(&puzzle, SolveMode::FirstSolution, &mut log);
        let stats = match &outcome {
            SolveOutcome::Solved { stats, .. } => *stats,
            _ => SolveStats::default(),
        };
        Replay::new(
            "6x6 easy".into(),
            &puzzle,
            log.events,
            stats,
            Duration::from_micros(58),
            true,
        )
    }

    #[test]
    fn k15_grid_lines_match_the_grid_shape() {
        let replay = sample();
        let lines = grid_lines(&replay);
        assert_eq!(lines.len(), 6);
        for line in &lines {
            assert_eq!(line.spans.len(), 6);
        }
    }

    #[test]
    fn k15_step_description_names_the_strategy() {
        let mut replay = sample();
        replay.step_forward();
        let text = step_description(&replay);
        assert!(text.contains("→ r"), "{text}");
        assert!(
            text.contains("FindDuo")
                || text.contains("AvoidTriple")
                || text.contains("FillByCount"),
            "{text}"
        );
    }

    #[test]
    fn k15_stats_line_reports_progress_and_grade() {
        let mut replay = sample();
        assert!(stats_line(&replay).contains("step 0/"));
        replay.jump_to_end();
        let line = stats_line(&replay);
        assert!(line.contains(&format!("step {}/{}", replay.steps(), replay.steps())));
        assert!(line.contains("easy"), "{line}");
    }

    #[test]
    fn k15_current_cell_is_highlighted_exactly_once() {
        let mut replay = sample();
        replay.step_forward();
        let lines = grid_lines(&replay);
        let highlighted: usize = lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .filter(|s| s.style.bg == Some(Color::Yellow))
                    .count()
            })
            .sum();
        assert_eq!(highlighted, 1, "exactly the touched cell is highlighted");
    }
}
