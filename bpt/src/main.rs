//! binsolve CLI — thin frontend over binsolve-core [K8-K12, K16, M1, M3].

#![forbid(unsafe_code)]

mod output;

use anyhow::{Context, Result, bail};
use bpt_core::event::{EventLog, NullObserver, Observer, format_trace};
use bpt_core::parse::{parse_corpus_file, parse_line};
use bpt_core::region::{PuzzleKind, Region, validate_givens, validate_solution};
use bpt_core::search::{SolveMode, SolveOutcome, solve};
use bpt_forge::carve::carve;
use bpt_forge::fill;
use bpt_forge::grade::Level;
use bpt_forge::rng;
use clap::Parser;
use output::{canonical_line, marker_line, terminal_display, write_atomic};
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

/// Exit codes (AR7/K12).
const EXIT_OK: u8 = 0;
const EXIT_SOME_FAILED: u8 = 1;
const EXIT_USAGE: u8 = 2;

#[derive(Parser, Debug)]
#[command(
    name = "bpt",
    version,
    about = "BinaryPuzzleToolkit — solve, generate and inspect binary puzzles",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Solve puzzles, prove uniqueness, explain the reasoning
    Solve(SolveArgs),
    /// Watch a puzzle being solved step by step
    Watch(WatchArgs),
    /// Generate puzzles with a proven-unique solution
    Forge(ForgeArgs),
}

#[derive(Parser, Debug)]
struct ForgeArgs {
    /// Puzzle type: a size like 8 for a plain n×n, or a tag such as
    /// 4x6x6, 4x8x8, 9x6x6, 8in14, 6in10in14
    #[arg(long, default_value = "8")]
    kind: String,

    /// How many puzzles to generate
    #[arg(long, default_value_t = 1)]
    count: u64,

    /// Seed; the same seed always produces the same puzzles
    #[arg(long, default_value_t = 0)]
    seed: u64,

    /// Hardest level to allow: L1, L2, L3 or L4
    #[arg(long, default_value = "L4")]
    level: String,

    /// Write the puzzles here instead of to standard output
    #[arg(long, value_name = "FILE")]
    out: Option<PathBuf>,

    /// Include each solution on its own `solution:` line
    #[arg(long)]
    with_solutions: bool,
}

/// Watching is delegated to the replay viewer, which owns its own
/// argument handling; only the shape needed to hand off lives here.
#[derive(Parser, Debug)]
struct WatchArgs {
    /// Puzzle to watch
    puzzle: Option<String>,
    /// Watch every puzzle in a file
    #[arg(long, value_name = "FILE", conflicts_with = "puzzle")]
    file: Option<PathBuf>,
    /// Replay speed in steps per second
    #[arg(long, default_value_t = 20)]
    speed: u64,
    /// Prove uniqueness while solving
    #[arg(long)]
    unique: bool,
}

#[derive(Parser, Debug)]
struct SolveArgs {
    /// Puzzle to solve, e.g. "1..0.0..." or "4x8x8:110..." (K8)
    puzzle: Option<String>,

    /// Read puzzles from a file, one per line; output maps 1:1 (K9)
    #[arg(long, value_name = "FILE", conflicts_with = "puzzle")]
    file: Option<PathBuf>,

    /// Write results to a file instead of stdout, written atomically (K10)
    #[arg(long, value_name = "FILE")]
    out: Option<PathBuf>,

    /// Print the solving steps: bare flag to stderr, --explain=FILE to a
    /// file. The equals sign is required so a puzzle argument is never
    /// mistaken for the trace target (K16, K16a).
    #[arg(
        long,
        value_name = "FILE",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "-"
    )]
    explain: Option<PathBuf>,

    /// Solve with human strategies only, never guess (M1)
    #[arg(long)]
    no_backtrack: bool,

    /// Prove the solution is unique instead of stopping at the first (K5)
    #[arg(long)]
    unique: bool,

    /// Verify puzzle+solution files instead of solving them (M3)
    #[arg(long)]
    check: bool,
}

fn main() -> ExitCode {
    match dispatch() {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("binsolve: {err:#}");
            ExitCode::from(EXIT_USAGE)
        }
    }
}

fn dispatch() -> Result<u8> {
    match Cli::parse().command {
        Command::Solve(args) => run(args),
        Command::Watch(args) => run_watch(args),
        Command::Forge(args) => run_forge(args),
    }
}

/// `bpt watch` re-execs the replay viewer, which is a separate binary
/// because it owns a terminal while it runs. Keeping it separate also
/// keeps the solving path free of any terminal machinery.
fn run_watch(args: WatchArgs) -> Result<u8> {
    let mut command = std::process::Command::new("bpt-tui");
    if let Some(puzzle) = &args.puzzle {
        command.arg(puzzle);
    }
    if let Some(file) = &args.file {
        command.arg("--file").arg(file);
    }
    command.arg("--speed").arg(args.speed.to_string());
    if args.unique {
        command.arg("--unique");
    }
    let status = command.status().context(
        "cannot start bpt-tui — it is built alongside bpt; \
         run `cargo build --release` or put it on PATH",
    )?;
    Ok(u8::try_from(status.code().unwrap_or(2)).unwrap_or(2))
}

fn run(args: SolveArgs) -> Result<u8> {
    if args.puzzle.is_none() && args.file.is_none() {
        bail!(
            "no puzzle given — pass a puzzle string, or --file FILE with one puzzle per line\n\
             example: bpt solve \"1..0.0..1...0..1.1....0....1.0..11..\""
        );
    }
    if args.check {
        return run_check(&args);
    }

    let inputs: Vec<String> = match (&args.puzzle, &args.file) {
        (Some(p), _) => vec![p.clone()],
        (_, Some(path)) => std::fs::read_to_string(path)
            .with_context(|| {
                format!(
                    "cannot read {} — check the path and permissions",
                    path.display()
                )
            })?
            .lines()
            .map(str::to_owned)
            // No filtering: K9 promises output line N describes input
            // line N, so a blank line keeps its slot and is reported as
            // invalid rather than silently shifting everything after it.
            .collect(),
        _ => unreachable!("guarded above"),
    };
    if inputs.is_empty() {
        bail!("input file is empty — add one puzzle per line");
    }

    let mode = if args.no_backtrack {
        SolveMode::StrategiesOnly
    } else if args.unique {
        SolveMode::ProveUniqueness
    } else {
        SolveMode::FirstSolution
    };
    // A batch trace on shared stderr would interleave into garbage;
    // sequential execution is the AR10 answer (M5 must respect this).
    let interactive = std::io::stdout().is_terminal() && inputs.len() == 1;

    let mut lines = Vec::with_capacity(inputs.len());
    let mut traces = String::new();
    let mut failures = 0usize;
    let mut pretty = String::new();

    for original in &inputs {
        let puzzle = match parse_line(original) {
            Ok(p) => p,
            Err(e) => {
                failures += 1;
                lines.push(marker_line("invalid", original));
                if args.explain.is_some() {
                    traces.push_str(&format!("{original}: {e}\n"));
                }
                continue;
            }
        };
        let mut log = EventLog::default();
        let mut null = NullObserver;
        let observer: &mut dyn Observer = if args.explain.is_some() {
            &mut log
        } else {
            &mut null
        };
        let started = Instant::now();
        let outcome = solve(&puzzle, mode, observer);
        let elapsed = started.elapsed();

        if !matches!(outcome, SolveOutcome::Solved { .. }) {
            failures += 1;
        }
        lines.push(canonical_line(&outcome, &puzzle, original));
        if args.explain.is_some() {
            if inputs.len() > 1 {
                traces.push_str(&format!("--- {original}\n"));
            }
            traces.push_str(&format_trace(&log.events));
        }
        if let Some(display) = terminal_display(
            &outcome,
            elapsed,
            mode == SolveMode::ProveUniqueness,
            interactive,
        ) {
            pretty.push_str(&display);
        }
    }

    let body = lines.join("\n") + "\n";
    match &args.out {
        Some(path) => write_atomic(path, &body)
            .with_context(|| format!("cannot write {} — check the path", path.display()))?,
        None => {
            if interactive {
                print!("{pretty}");
            }
            print!("{body}");
            std::io::stdout().flush().ok();
        }
    }
    if let Some(target) = &args.explain {
        if target.as_os_str() == "-" {
            eprint!("{traces}");
        } else {
            write_atomic(target, &traces)
                .with_context(|| format!("cannot write {}", target.display()))?;
        }
    }

    Ok(if failures == 0 {
        EXIT_OK
    } else {
        EXIT_SOME_FAILED
    })
}

/// Generate puzzles. Every one is carved from a filled grid with a
/// uniqueness proof after each removed clue, so what comes out has
/// exactly one solution — the same guarantee a published puzzle carries.
fn run_forge(args: ForgeArgs) -> Result<u8> {
    let (n, regions) = geometry_for(&args.kind)?;
    let ceiling = match args.level.to_uppercase().as_str() {
        "L1" => Level::L1,
        "L2" => Level::L2,
        "L3" => Level::L3,
        "L4" => Level::L4,
        other => bail!(
            "unknown level {other:?} — use L1, L2, L3 or L4 \
             (L1 needs only local patterns, L4 allows guessing)"
        ),
    };

    let mut lines = Vec::new();
    for index in 0..args.count {
        let mut rng = rng::stream(args.seed, index, 0);
        let Some(solution) = fill::solution(n, &regions, &mut rng) else {
            bail!(
                "the {} geometry has no solution at all — it is over-constrained, \
                 so no seed will help",
                args.kind
            );
        };
        let carved = carve(&solution, &regions, ceiling, &mut rng);
        let tag = tag_for(&args.kind);
        lines.push(format!("{tag}{}", carved.puzzle.to_line()));
        if args.with_solutions {
            lines.push(format!("solution:{}", carved.solution.to_line()));
        }
    }

    let body = lines.join("\n") + "\n";
    match &args.out {
        Some(path) => write_atomic(path, &body)
            .with_context(|| format!("cannot write {} — check the path", path.display()))?,
        None => {
            print!("{body}");
            std::io::stdout().flush().ok();
        }
    }
    Ok(EXIT_OK)
}

/// Map a `--kind` argument onto a grid size and its regions. Plain sizes
/// and the five tags are built in; the tag travels with the output so a
/// generated puzzle reads back exactly like a published one.
fn geometry_for(kind: &str) -> Result<(usize, Vec<Region>)> {
    if let Ok(n) = kind.parse::<usize>() {
        if n < 4 || n % 2 != 0 {
            bail!("size {n} is not usable — use an even size of at least 4");
        }
        return Ok((n, vec![Region::square(0, 0, n)]));
    }
    let puzzle_kind = PuzzleKind::from_tag(kind).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown type {kind:?} — use a size like 8, or one of \
             4x6x6, 4x8x8, 9x6x6, 8in14, 6in10in14"
        )
    })?;
    Ok((puzzle_kind.grid_size(), puzzle_kind.regions()))
}

fn tag_for(kind: &str) -> String {
    match PuzzleKind::from_tag(kind) {
        Some(k) => k.tag().map(|t| format!("{t}:")).unwrap_or_default(),
        None => String::new(),
    }
}

/// M3: verify corpus-format files (puzzle + solution) instead of solving.
fn run_check(args: &SolveArgs) -> Result<u8> {
    let Some(path) = &args.file else {
        bail!("--check needs --file FILE with a puzzle line and a 'solution:' line");
    };
    let content =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let (puzzle, solution) = parse_corpus_file(&content)
        .with_context(|| format!("{} is not a valid puzzle/solution file", path.display()))?;
    let Some(solution) = solution else {
        bail!(
            "{} has no 'solution:' line — --check needs a solution to verify",
            path.display()
        );
    };
    let mut problems: Vec<String> = validate_givens(&puzzle.givens, &solution)
        .iter()
        .map(|v| v.to_string())
        .collect();
    problems.extend(
        validate_solution(&solution, &puzzle.regions())
            .iter()
            .map(|v| v.to_string()),
    );
    if problems.is_empty() {
        println!("ok: {}", path.display());
        Ok(EXIT_OK)
    } else {
        for p in &problems {
            println!("invalid: {p}");
        }
        Ok(EXIT_SOME_FAILED)
    }
}
