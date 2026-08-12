//! binsolve-tui — watch puzzles being solved [K15].
//!
//! Solves at full speed into an event log, then replays it at your
//! chosen pace (AR9): the timing statistics stay honest and the render
//! model is testable without a terminal.

#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail};
use binsolve_core::event::EventLog;
use binsolve_core::parse::parse_line;
use binsolve_core::search::{SolveMode, SolveOutcome, SolveStats, solve};
use binsolve_tui::{replay, ui};
use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::*;
use replay::Replay;
use std::io::stdout;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use ui::ViewState;

#[derive(Parser, Debug)]
#[command(
    name = "binsolve-tui",
    version,
    about = "Watch binary puzzles being solved, step by step"
)]
struct Args {
    /// Puzzle to watch, e.g. "1..0.0..." or "4x8x8:110..."
    puzzle: Option<String>,

    /// Watch every puzzle in a file, one per line
    #[arg(long, value_name = "FILE", conflicts_with = "puzzle")]
    file: Option<PathBuf>,

    /// Replay speed in steps per second
    #[arg(long, default_value_t = 20)]
    speed: u64,

    /// Prove uniqueness while solving (shows the extra search)
    #[arg(long)]
    unique: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let inputs: Vec<String> = match (&args.puzzle, &args.file) {
        (Some(p), _) => vec![p.clone()],
        (_, Some(path)) => std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?
            .lines()
            .map(str::to_owned)
            .filter(|l| !l.trim().is_empty())
            .collect(),
        _ => {
            bail!("no puzzle given — pass a puzzle string, or --file FILE with one puzzle per line")
        }
    };
    if inputs.is_empty() {
        bail!("input file has no puzzles — add one puzzle per line");
    }

    let mode = if args.unique {
        SolveMode::ProveUniqueness
    } else {
        SolveMode::FirstSolution
    };
    let mut replays = Vec::with_capacity(inputs.len());
    for (i, line) in inputs.iter().enumerate() {
        let puzzle = parse_line(line).with_context(|| format!("puzzle {} is not valid", i + 1))?;
        let mut log = EventLog::default();
        let started = Instant::now();
        let outcome = solve(&puzzle, mode, &mut log);
        let elapsed = started.elapsed();
        let (stats, solved) = match &outcome {
            SolveOutcome::Solved { stats, .. } => (*stats, true),
            _ => (SolveStats::default(), false),
        };
        let name = match puzzle.kind.tag() {
            Some(tag) => format!("{tag} #{}", i + 1),
            None => format!("{n}x{n} #{}", i + 1, n = puzzle.givens.size()),
        };
        replays.push(Replay::new(
            name, &puzzle, log.events, stats, elapsed, solved,
        ));
    }

    run_tui(replays, args.speed.max(1))
}

fn run_tui(replays: Vec<Replay>, speed: u64) -> Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;

    let result = event_loop(&mut terminal, replays, speed);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn event_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    mut replays: Vec<Replay>,
    speed: u64,
) -> Result<()> {
    let mut selected = 0usize;
    let mut playing = true;
    let mut speed = speed;
    let mut last_step = Instant::now();

    loop {
        terminal.draw(|frame| {
            ui::render(
                frame,
                &ViewState {
                    replays: &replays,
                    selected,
                    playing,
                    speed,
                },
            )
        })?;

        let tick = Duration::from_micros(1_000_000 / speed.max(1));
        if event::poll(Duration::from_millis(10))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char(' ') => playing = !playing,
                KeyCode::Right => {
                    playing = false;
                    replays[selected].step_forward();
                }
                KeyCode::Left => {
                    playing = false;
                    replays[selected].step_back();
                }
                KeyCode::Down => {
                    selected = (selected + 1) % replays.len();
                }
                KeyCode::Up => {
                    selected = (selected + replays.len() - 1) % replays.len();
                }
                KeyCode::Char('+') | KeyCode::Char('=') => speed = (speed * 2).min(2000),
                KeyCode::Char('-') => speed = (speed / 2).max(1),
                KeyCode::Home => replays[selected].jump_to_start(),
                KeyCode::End => replays[selected].jump_to_end(),
                _ => {}
            }
        }

        if playing && last_step.elapsed() >= tick {
            last_step = Instant::now();
            if !replays[selected].step_forward() {
                playing = false;
            }
        }
    }
    Ok(())
}
