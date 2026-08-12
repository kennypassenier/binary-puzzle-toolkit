//! K15: render a real frame through ratatui's in-memory backend, so
//! the full layout is exercised without a terminal.

use binsolve_core::event::EventLog;
use binsolve_core::parse::parse_line;
use binsolve_core::search::{SolveMode, SolveOutcome, SolveStats, solve};
use binsolve_tui::replay::Replay;
use binsolve_tui::ui::{ViewState, render};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::time::Duration;

fn replay_of(name: &str, line: &str) -> Replay {
    let puzzle = parse_line(line).expect("valid puzzle");
    let mut log = EventLog::default();
    let outcome = solve(&puzzle, SolveMode::FirstSolution, &mut log);
    let (stats, solved) = match &outcome {
        SolveOutcome::Solved { stats, .. } => (*stats, true),
        _ => (SolveStats::default(), false),
    };
    Replay::new(
        name.into(),
        &puzzle,
        log.events,
        stats,
        Duration::from_micros(58),
        solved,
    )
}

fn frame_text(replays: &[Replay], selected: usize) -> String {
    let backend = TestBackend::new(90, 20);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    terminal
        .draw(|frame| {
            render(
                frame,
                &ViewState {
                    replays,
                    selected,
                    playing: true,
                    speed: 20,
                },
            )
        })
        .expect("draw");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_string())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn k15_full_frame_renders_all_panels() {
    let mut replays = vec![
        replay_of("6x6 easy #1", "1..0....00.1.00..1......00.1...1..00"),
        replay_of("6x6 empty #2", &".".repeat(36)),
    ];
    replays[0].step_forward();
    let text = frame_text(&replays, 0);
    println!("{text}");

    assert!(text.contains("binsolve"), "title bar missing:\n{text}");
    assert!(text.contains("6x6 easy #1"), "puzzle name missing");
    assert!(text.contains("(1/2 puzzles)"), "puzzle counter missing");
    assert!(text.contains("solved"), "solved state missing");
    assert!(text.contains("playing at 20 steps/s"), "play state missing");
    assert!(text.contains("grid"), "grid panel missing");
    assert!(text.contains("current step"), "step panel missing");
    assert!(text.contains("deductions"), "stats missing");
    assert!(text.contains("q quit"), "key help missing");
}

#[test]
fn k15_frame_survives_the_largest_grid() {
    // 18x18 (9 times 6x6) in a small terminal must not panic.
    let content = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../corpus/special/9x6x6/bp-2525-20260809-veryhard.txt"),
    )
    .expect("corpus file");
    let line = content.lines().next().expect("puzzle line");
    let replays = vec![replay_of("9x6x6", line)];
    let text = frame_text(&replays, 0);
    assert!(text.contains("9x6x6"));
}

#[test]
fn k15_unsolvable_puzzle_is_labelled() {
    let replays = vec![replay_of("bad", &format!("000{}", ".".repeat(33)))];
    let text = frame_text(&replays, 0);
    assert!(
        text.contains("no unique solution"),
        "contradictory puzzle must be labelled:\n{text}"
    );
}
