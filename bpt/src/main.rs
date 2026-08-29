//! binsolve CLI — thin frontend over binsolve-core [K8-K12, K16, M1, M3].

#![forbid(unsafe_code)]

mod atomic;
mod output;

use anyhow::{Context, Result, bail};
use bpt_core::event::{EventLog, NullObserver, Observer, format_trace};
use bpt_core::parse::{parse_corpus_file, parse_line};
use bpt_core::region::{PuzzleKind, Region, validate_givens, validate_solution};
use bpt_core::search::{SolveMode, SolveOutcome, solve};
use bpt_forge::batch::{self, Outcome, Plan, Shortfall};
use bpt_forge::grade::Level;
use bpt_forge::manifest as forge_manifest;
use clap::Parser;
use output::{canonical_line, marker_line, terminal_display, write_atomic};
use std::collections::HashSet;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

/// Exit codes (AR7/K12).
const EXIT_OK: u8 = 0;
const EXIT_SOME_FAILED: u8 = 1;
const EXIT_USAGE: u8 = 2;

/// Names inside a batch directory. Fixed rather than configurable: the
/// validation harness and `bpt solve --file` both look for them.
const FLAT_FILE: &str = "puzzles.txt";
const MANIFEST_FILE: &str = "manifest.toml";

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

    /// Write a full batch here: one two-line file per puzzle, a flat
    /// `puzzles.txt` for `bpt solve --file … --unique`, and a manifest
    #[arg(long, value_name = "DIR", conflicts_with = "out")]
    out_dir: Option<PathBuf>,

    /// Write into a directory that already holds a batch. Without this
    /// a run refuses, because a batch owns its directory (AR29)
    #[arg(long, requires = "out_dir")]
    force: bool,
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
    let ceiling = level_for(&args.level)?;
    let plan = Plan::new(n, regions, ceiling, args.seed, args.count);

    match &args.out_dir {
        Some(dir) => forge_batch(&args, &plan, dir),
        None => forge_stream(&args, &plan),
    }
}

fn level_for(level: &str) -> Result<Level> {
    match level.to_uppercase().as_str() {
        "L1" => Ok(Level::L1),
        "L2" => Ok(Level::L2),
        "L3" => Ok(Level::L3),
        "L4" => Ok(Level::L4),
        other => bail!(
            "unknown level {other:?} — use L1, L2, L3 or L4 \
             (L1 needs only local patterns, L4 allows guessing)"
        ),
    }
}

/// A shortfall is not an error: the puzzles that were produced are
/// valid, so they are emitted and the exit code carries the news
/// (AR30). Only an unsolvable geometry aborts, because no seed helps.
fn check_geometry(outcome: &Outcome, kind: &str) -> Result<()> {
    if outcome.shortfalls.contains(&Shortfall::GeometryUnsolvable) {
        bail!(
            "the {kind} geometry has no solution at all — it is over-constrained, \
             so no seed will help"
        );
    }
    Ok(())
}

/// `--out` / stdout: one line per puzzle, exactly binsolve's format.
fn forge_stream(args: &ForgeArgs, plan: &Plan) -> Result<u8> {
    let outcome = batch::run(plan, &mut HashSet::new());
    check_geometry(&outcome, &args.kind)?;

    let tag = tag_for(&args.kind);
    let mut lines = Vec::new();
    for produced in &outcome.produced {
        lines.push(format!("{tag}{}", produced.carved.puzzle.to_line()));
        if args.with_solutions {
            lines.push(format!("solution:{}", produced.carved.solution.to_line()));
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
    report_shortfalls(&outcome);
    Ok(exit_for(&outcome))
}

/// K27/AR29/AR30: a whole batch into its own directory.
///
/// All-or-nothing is implemented by generating everything in memory
/// first and only then touching the disk: with a batch of a few hundred
/// small grids there is nothing to stream, and a failure before the
/// first write leaves no half-batch to clean up.
fn forge_batch(args: &ForgeArgs, plan: &Plan, dir: &Path) -> Result<u8> {
    let existing = existing_puzzles(dir)?;
    if !existing.is_empty() && !args.force {
        bail!(
            "{} already holds {} puzzle(s) — a batch owns its directory. \
             Use --force to add to it, or pick an empty directory",
            dir.display(),
            existing.len()
        );
    }
    // AR28: a run into a populated directory stays deterministic, but
    // reproduces differently than into an empty one, so the starting
    // set is part of the batch's identity.
    let mut seen = existing;

    let started = Instant::now();
    // M26: progress on a terminal, silence in a pipe. It goes to stderr
    // so redirecting the batch never captures it (K16a), and it is
    // rewritten in place so a 4000-puzzle run leaves one line, not 4000.
    let show_progress = std::io::stderr().is_terminal();
    let kind = args.kind.clone();
    let outcome = batch::run_with(plan, &mut seen, &mut |done, total| {
        if show_progress {
            eprint!(
                "\rforging {kind}: {done}/{total} ({:.1}s)   ",
                started.elapsed().as_secs_f64()
            );
            std::io::stderr().flush().ok();
        }
    });
    if show_progress {
        eprintln!();
    }
    check_geometry(&outcome, &args.kind)?;
    let elapsed = started.elapsed();

    fs::create_dir_all(dir)
        .with_context(|| format!("cannot create {} — check the path", dir.display()))?;
    atomic::clean_orphans(dir)?;

    let tag = tag_for(&args.kind);
    let mut entries = Vec::new();
    let mut flat = String::new();
    for produced in &outcome.produced {
        let carved = &produced.carved;
        let line = format!("{tag}{}", carved.puzzle.to_line());
        let name = format!(
            "bf-{}-{}-{}.txt",
            plan.seed,
            produced.index,
            carved.level.name()
        );
        // The corpus files are two-line so `--check` can verify them;
        // the flat file is one-line so `--unique` can prove them (AR27).
        atomic::write(
            &dir.join(&name),
            &format!("{line}\nsolution:{}\n", carved.solution.to_line()),
        )?;
        flat.push_str(&line);
        flat.push('\n');
        entries.push(forge_manifest::Entry {
            file: name,
            index: produced.index,
            attempt: produced.attempt,
            level: carved.level,
            clues: carved.clues,
            digest: forge_manifest::digest(&line),
        });
    }
    atomic::write(&dir.join(FLAT_FILE), &flat)?;

    // AR29: fsync the directory before the manifest, so a crash can
    // never leave a manifest promising files that are not there yet.
    atomic::sync_dir(dir)?;
    let manifest = forge_manifest::Manifest {
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        tool_revision: option_env!("BPT_GIT_REV").unwrap_or("unknown").to_string(),
        grading_version: forge_manifest::GRADING_VERSION,
        kind: args.kind.clone(),
        grid_size: plan.n,
        level_ceiling: plan.ceiling,
        seed: plan.seed,
        requested: plan.count,
        completed: outcome.produced.len() as u64,
        status: if outcome.complete() {
            forge_manifest::Status::Complete
        } else {
            forge_manifest::Status::Partial
        },
        elapsed_ms: elapsed.as_millis() as u64,
        notes: outcome.shortfalls.iter().map(describe).collect(),
        puzzles: entries,
    };
    atomic::write(
        &dir.join(MANIFEST_FILE),
        &manifest.to_toml().context("cannot render the manifest")?,
    )?;
    atomic::sync_dir(dir)?;

    if std::io::stdout().is_terminal() {
        println!(
            "{} puzzle(s) in {} ({} ms)",
            manifest.completed,
            dir.display(),
            manifest.elapsed_ms
        );
    }
    report_shortfalls(&outcome);
    Ok(exit_for(&outcome))
}

/// Read back the puzzle lines already in a directory, so a `--force` run
/// adds only what is new (M21). Reads the flat file rather than the
/// per-puzzle files: it holds exactly the lines the duplicate check
/// compares, in one read.
fn existing_puzzles(dir: &Path) -> Result<HashSet<String>> {
    let flat = dir.join(FLAT_FILE);
    if !flat.exists() {
        return Ok(HashSet::new());
    }
    let text = fs::read_to_string(&flat)
        .with_context(|| format!("cannot read {} — remove it to start fresh", flat.display()))?;
    Ok(text
        .lines()
        .map(|line| line.split_once(':').map_or(line, |(_, rest)| rest))
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

fn describe(shortfall: &Shortfall) -> String {
    match shortfall {
        Shortfall::GeometryUnsolvable => "the geometry has no solution".to_string(),
        Shortfall::OnlyDuplicates { index } => {
            format!("puzzle {index}: every attempt reproduced a puzzle already generated")
        }
    }
}

/// Shortfalls go to stderr so a piped batch stays clean (K16a).
fn report_shortfalls(outcome: &Outcome) {
    for shortfall in &outcome.shortfalls {
        eprintln!("bpt forge: {}", describe(shortfall));
    }
}

fn exit_for(outcome: &Outcome) -> u8 {
    if outcome.complete() {
        EXIT_OK
    } else {
        EXIT_SOME_FAILED
    }
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
