//! binsolve CLI — thin frontend over binsolve-core [K8-K12, K16, M1, M3].

#![forbid(unsafe_code)]

mod output;

use anyhow::{Context, Result, bail};
use binsolve_core::event::{EventLog, NullObserver, Observer, format_trace};
use binsolve_core::parse::{parse_corpus_file, parse_line};
use binsolve_core::region::{validate_givens, validate_solution};
use binsolve_core::search::{SolveMode, SolveOutcome, solve};
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
    name = "binsolve",
    version,
    about = "Solve binary puzzles (Takuzu/Binairo), including binarypuzzle.com's five special types",
    long_about = None
)]
struct Args {
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
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("binsolve: {err:#}");
            ExitCode::from(EXIT_USAGE)
        }
    }
}

fn run() -> Result<u8> {
    let args = Args::parse();
    if args.puzzle.is_none() && args.file.is_none() {
        bail!(
            "no puzzle given — pass a puzzle string, or --file FILE with one puzzle per line\n\
             example: binsolve \"1..0.0..1...0..1.1....0....1.0..11..\""
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

/// M3: verify corpus-format files (puzzle + solution) instead of solving.
fn run_check(args: &Args) -> Result<u8> {
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
