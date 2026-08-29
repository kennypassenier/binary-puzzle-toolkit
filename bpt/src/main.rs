//! binsolve CLI — thin frontend over binsolve-core [K8-K12, K16, M1, M3].

#![forbid(unsafe_code)]

mod atomic;
mod output;
mod parallel;

use anyhow::{Context, Result, bail};
use bpt_core::event::{EventLog, NullObserver, Observer, format_trace};
use bpt_core::parse::{parse_corpus_file, parse_line};
use bpt_core::region::{
    Puzzle, PuzzleKind, Region, compose_from_tag, validate_givens, validate_solution,
};
use bpt_core::search::{SolveMode, SolveOutcome, solve};
use bpt_forge::batch::{self, Outcome, Plan, Shortfall};
use bpt_forge::carve::Symmetry;
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
const MANIFEST_FILE: &str = "manifest.json";

/// AR29b: a cancelled batch is complete and valid but shorter than
/// asked for, which no consumer may mistake for a finished run.
const EXIT_CANCELLED: u8 = 3;

/// B5: the release number alone cannot identify a build, and the batch
/// manifests record this exact string.
const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), " (", env!("BPT_GIT_REV"), ")");

#[derive(Parser, Debug)]
#[command(
    name = "bpt",
    version = VERSION,
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
    /// Show what a geometry file describes, and whether it is valid
    Inspect(InspectArgs),
}

/// M25: the command the help text has promised since the merge.
#[derive(Parser, Debug)]
struct InspectArgs {
    /// A geometry file, or the name of a built-in type such as 4x8x8
    #[arg(value_name = "GEOMETRY")]
    geometry: String,
}

#[derive(Parser, Debug)]
struct ForgeArgs {
    /// Puzzle type: a size like 8 for a plain n×n, or a tag such as
    /// 4x6x6, 4x8x8, 9x6x6, 8in14, 6in10in14
    #[arg(long, default_value = "8")]
    kind: String,

    /// Generate an invented type from a geometry file (K23/K28) instead
    /// of a built-in one
    #[arg(long, value_name = "FILE", conflicts_with = "kind")]
    geometry: Option<PathBuf>,

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

    /// Stop carving once this many clues remain, instead of removing as
    /// much as the level allows (M24)
    #[arg(long, value_name = "N")]
    clues: Option<usize>,

    /// Lay the clues out symmetrically: none, rotational (a half turn)
    /// or mirror (left to right). Symmetry costs clues and time (M24)
    #[arg(long, default_value = "none")]
    symmetry: String,
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

    /// Read the regions from a geometry file, for a type that has no tag
    /// in the line format yet (K28)
    #[arg(long, value_name = "FILE")]
    geometry: Option<PathBuf>,
}

fn main() -> ExitCode {
    match dispatch() {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("bpt: {err:#}");
            ExitCode::from(EXIT_USAGE)
        }
    }
}

fn dispatch() -> Result<u8> {
    match Cli::parse().command {
        Command::Solve(args) => run(args),
        Command::Watch(args) => run_watch(args),
        Command::Forge(args) => run_forge(args),
        Command::Inspect(args) => run_inspect(args),
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

    let supplied_regions = match &args.geometry {
        Some(path) => {
            let geometry = read_geometry(path)?;
            Some((geometry.size, geometry.to_regions()))
        }
        None => None,
    };

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
        // A geometry file replaces the regions the tag implied. It is
        // the only way to solve a type the line format has no tag for.
        //
        // A geometry that does not fit the grid is refused here rather
        // than allowed through: its regions would index past the end of
        // the grid, which is a panic, not a wrong answer.
        let puzzle = match &supplied_regions {
            Some((size, regions)) => {
                if puzzle.givens.size() != *size {
                    failures += 1;
                    lines.push(marker_line("invalid", original));
                    if args.explain.is_some() {
                        traces.push_str(&format!(
                            "{original}: the geometry describes a {size}x{size} grid, \
                             this puzzle is {}x{}\n",
                            puzzle.givens.size(),
                            puzzle.givens.size()
                        ));
                    }
                    continue;
                }
                Puzzle::custom(puzzle.givens.clone(), regions.clone())
            }
            None => puzzle,
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
    let (n, regions) = match &args.geometry {
        Some(path) => {
            let geometry = read_geometry(path)?;
            (geometry.size, geometry.to_regions())
        }
        None => geometry_for(&args.kind)?,
    };
    let ceiling = level_for(&args.level)?;
    let mut plan = Plan::new(n, regions, ceiling, args.seed, args.count);
    plan.symmetry = symmetry_for(&args.symmetry)?;
    plan.target_clues = args.clues;
    if args.count == 0 {
        bail!("--count 0 would generate nothing — ask for at least one puzzle");
    }
    if let Some(target) = args.clues {
        let cells = n * n;
        if target >= cells {
            bail!(
                "a {n}x{n} grid has {cells} cells — asking to keep {target} clues would leave \
                 the solution, not a puzzle"
            );
        }
    }

    match &args.out_dir {
        Some(dir) => forge_batch(&args, &plan, dir),
        None => forge_stream(&args, &plan),
    }
}

/// What to call the geometry in a message. `--kind` keeps its default
/// even when `--geometry` was given, so reporting it would name a type
/// the run never touched — found by an over-constrained geometry file
/// being blamed on "the 8 geometry".
fn geometry_name(args: &ForgeArgs) -> String {
    match &args.geometry {
        Some(path) => path.display().to_string(),
        None => args.kind.clone(),
    }
}

fn symmetry_for(name: &str) -> Result<Symmetry> {
    match name.to_lowercase().as_str() {
        "none" => Ok(Symmetry::None),
        "rotational" => Ok(Symmetry::Rotational),
        "mirror" => Ok(Symmetry::Mirror),
        other => bail!("unknown symmetry {other:?} — use none, rotational or mirror"),
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
    let outcome = batch::run_from(
        plan,
        &mut HashSet::new(),
        &mut parallel::OnAllCores,
        &mut |_, _| {},
    );
    check_geometry(&outcome, &geometry_name(args))?;

    let tag = emitted_tag(args);
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
    report_clue_target(&outcome, plan.target_clues);
    Ok(exit_for(&outcome))
}

/// K27/AR29/AR30: a whole batch into its own directory.
///
/// All-or-nothing is implemented by generating everything in memory
/// first and only then touching the disk: with a batch of a few hundred
/// small grids there is nothing to stream, and a failure before the
/// first write leaves no half-batch to clean up.
fn forge_batch(args: &ForgeArgs, plan: &Plan, dir: &Path) -> Result<u8> {
    let previous = existing_batch(dir)?;
    if let Some((manifest, lines)) = &previous {
        if !args.force {
            bail!(
                "{} already holds {} puzzle(s) — a batch owns its directory. \
                 Use --force to add to it, or pick an empty directory",
                dir.display(),
                lines.len()
            );
        }
        // One directory, one geometry. Two would leave a manifest that
        // cannot describe its own contents with one set of fields, and
        // the restore drill rebuilds every puzzle from those fields.
        let wanted = geometry_name(args);
        if manifest.kind != wanted || manifest.level_ceiling != plan.ceiling {
            bail!(
                "{} holds {} puzzles at level {}, and this run would add {} at {} — \
                 one directory holds one geometry at one level. \
                 Use a different directory",
                dir.display(),
                manifest.kind,
                manifest.level_ceiling.name(),
                wanted,
                plan.ceiling.name()
            );
        }
    }
    // AR28: a run into a populated directory stays deterministic, but
    // reproduces differently than into an empty one, so the starting
    // set is part of the batch's identity.
    let mut seen: HashSet<String> = previous
        .iter()
        .flat_map(|(_, lines)| lines.iter().map(|line| untagged(line).to_string()))
        .collect();

    let started = Instant::now();
    // M26: progress on a terminal, silence in a pipe. It goes to stderr
    // so redirecting the batch never captures it (K16a), and it is
    // rewritten in place so a 4000-puzzle run leaves one line, not 4000.
    let show_progress = std::io::stderr().is_terminal();
    let kind = args.kind.clone();
    // M26/AR29b: Ctrl-C finishes the puzzle in flight and writes what is
    // done. A second Ctrl-C is left to the default handler, so a run that
    // will not stop can still be killed.
    let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = std::sync::Arc::clone(&cancelled);
    let installed = ctrlc::set_handler(move || {
        if flag.swap(true, std::sync::atomic::Ordering::SeqCst) {
            std::process::exit(EXIT_CANCELLED.into());
        }
        eprintln!(
            "\nbpt forge: cancelling — finishing the current puzzle, press Ctrl-C again to abort"
        );
    })
    .is_ok();
    let outcome = batch::run_until(
        plan,
        &mut seen,
        &mut parallel::OnAllCores,
        &mut |done, total| {
            if show_progress {
                eprint!(
                    "\rforging {kind}: {done}/{total} ({:.1}s)   ",
                    started.elapsed().as_secs_f64()
                );
                std::io::stderr().flush().ok();
            }
        },
        &|| installed && cancelled.load(std::sync::atomic::Ordering::SeqCst),
    );
    if show_progress {
        eprintln!();
    }
    check_geometry(&outcome, &geometry_name(args))?;
    let elapsed = started.elapsed();

    fs::create_dir_all(dir)
        .with_context(|| format!("cannot create {} — check the path", dir.display()))?;
    atomic::clean_orphans(dir)?;

    // AR29: a batch commits completely or not at all. A crash cannot
    // clean up after itself — that is what writing the manifest last
    // protects against — but an error can, and leaving a directory of
    // puzzles with no manifest behind would look like a batch to
    // everything except a reader that checks.
    let written = match write_batch(args, plan, dir, &outcome, elapsed, previous.as_ref()) {
        Ok(written) => written,
        Err(err) => {
            discard_batch(dir, &outcome, plan);
            return Err(err);
        }
    };

    if std::io::stdout().is_terminal() {
        println!(
            "{} puzzle(s) in {} ({} ms)",
            written.completed,
            dir.display(),
            written.elapsed_ms
        );
    }
    report_shortfalls(&outcome);
    report_clue_target(&outcome, plan.target_clues);
    Ok(exit_for(&outcome))
}

/// Write the whole batch: one corpus file per puzzle, the flat file the
/// solver validates, then the manifest — in that order, each fsynced
/// before the next depends on it (AR29).
#[allow(clippy::too_many_arguments)]
fn write_batch(
    args: &ForgeArgs,
    plan: &Plan,
    dir: &Path,
    outcome: &Outcome,
    elapsed: std::time::Duration,
    previous: Option<&(forge_manifest::Manifest, Vec<String>)>,
) -> Result<forge_manifest::Manifest> {
    let tag = emitted_tag(args);
    // G1: what was already here comes along. Before this, a --force run
    // rewrote both the flat file and the manifest with only its own
    // puzzles, so earlier ones stayed on disk while disappearing from
    // everything that describes the directory — validation checked two
    // of five, and the restore drill restored two of five.
    let (mut entries, mut flat) = match previous {
        Some((manifest, lines)) => {
            let mut flat = String::new();
            for line in lines {
                flat.push_str(line);
                flat.push('\n');
            }
            (manifest.puzzles.clone(), flat)
        }
        None => (Vec::new(), String::new()),
    };
    for produced in &outcome.produced {
        let carved = &produced.carved;
        let line = format!("{tag}{}", carved.puzzle.to_line());
        let name = batch_file_name(plan.seed, produced.index, carved.level);
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
            seed: plan.seed,
            index: produced.index,
            attempt: produced.attempt,
            level: carved.level,
            clues: carved.clues,
            digest: forge_manifest::digest(&line),
            budget_hits: carved.budget_hits,
        });
    }
    atomic::write(&dir.join(FLAT_FILE), &flat)?;

    // AR29: fsync the directory before the manifest, so a crash can
    // never leave a manifest promising files that are not there yet.
    atomic::sync_dir(dir)?;
    let manifest = forge_manifest::Manifest {
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        tool_revision: env!("BPT_GIT_REV").to_string(),
        grading_version: forge_manifest::GRADING_VERSION,
        kind: args.kind.clone(),
        grid_size: plan.n,
        level_ceiling: plan.ceiling,
        requested: plan.count,
        completed: outcome.produced.len() as u64,
        status: if outcome.complete() {
            forge_manifest::Status::Complete
        } else if outcome.cancelled() {
            forge_manifest::Status::Cancelled
        } else {
            forge_manifest::Status::Partial
        },
        elapsed_ms: elapsed.as_millis() as u64,
        notes: outcome.shortfalls.iter().map(describe).collect(),
        puzzles: entries,
    };
    atomic::write(
        &dir.join(MANIFEST_FILE),
        &manifest.to_json().context("cannot render the manifest")?,
    )?;
    atomic::sync_dir(dir)?;

    Ok(manifest)
}

/// Undo a failed batch. Only the files this run would have written are
/// removed, by name, so a `--force` run that fails cannot take the
/// existing batch down with it.
fn discard_batch(dir: &Path, outcome: &Outcome, plan: &Plan) {
    for produced in &outcome.produced {
        let name = batch_file_name(plan.seed, produced.index, produced.carved.level);
        let _ = fs::remove_file(dir.join(name));
    }
    let _ = atomic::clean_orphans(dir);
    // The flat file and the manifest describe this run only, so they go
    // whether or not this run created them: a stale pair left next to a
    // half-removed batch is worse than none.
    let _ = fs::remove_file(dir.join(FLAT_FILE));
    let _ = fs::remove_file(dir.join(MANIFEST_FILE));
}

/// AR30: every file names what reproduces it.
fn batch_file_name(seed: u64, index: u64, level: Level) -> String {
    format!("bf-{seed}-{index}-{}.txt", level.name())
}

/// What a directory already holds: its manifest, and the puzzle line of
/// every file that manifest names.
///
/// The manifest is the authority rather than the flat file, because the
/// manifest is what says which files belong to the batch. A file it
/// names that has gone missing is an error, not something to work
/// around: the directory and its description have to agree before a run
/// adds to them.
fn existing_batch(dir: &Path) -> Result<Option<(forge_manifest::Manifest, Vec<String>)>> {
    let path = dir.join(MANIFEST_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("cannot read {} — remove it to start fresh", path.display()))?;
    let manifest = forge_manifest::Manifest::from_json(&text)
        .with_context(|| format!("{} is not a readable manifest", path.display()))?;

    let mut lines = Vec::with_capacity(manifest.puzzles.len());
    for entry in &manifest.puzzles {
        let body = fs::read_to_string(dir.join(&entry.file)).with_context(|| {
            format!(
                "{} lists {} but that file is gone — the directory and its manifest \
                 disagree, so nothing was added",
                path.display(),
                entry.file
            )
        })?;
        let line = body.lines().next().unwrap_or_default().to_string();
        lines.push(line);
    }
    Ok(Some((manifest, lines)))
}

/// The grid part of a puzzle line, without any tag.
fn untagged(line: &str) -> &str {
    line.split_once(':').map_or(line, |(_, rest)| rest)
}

fn describe(shortfall: &Shortfall) -> String {
    match shortfall {
        Shortfall::GeometryUnsolvable => "the geometry has no solution".to_string(),
        Shortfall::OnlyDuplicates { index } => {
            format!("puzzle {index}: every attempt reproduced a puzzle already generated")
        }
        Shortfall::Cancelled { after } => {
            format!("cancelled after {after} of the requested puzzles")
        }
    }
}

/// Shortfalls go to stderr so a piped batch stays clean (K16a).
fn report_shortfalls(outcome: &Outcome) {
    for shortfall in &outcome.shortfalls {
        eprintln!("bpt forge: {}", describe(shortfall));
    }
}

/// M24: a clue target that could not be met is reported, never silently
/// missed. It is not a shortfall of the batch — every puzzle in it is
/// valid — so it does not change the exit code.
fn report_clue_target(outcome: &Outcome, target: Option<usize>) {
    let Some(target) = target else { return };
    let missed = outcome
        .produced
        .iter()
        .filter(|p| p.carved.clues > target)
        .count();
    if missed > 0 {
        let worst = outcome
            .produced
            .iter()
            .map(|p| p.carved.clues)
            .max()
            .unwrap_or(target);
        eprintln!(
            "bpt forge: {missed} of {} puzzles could not reach {target} clues \
             (uniqueness stopped the carve; the heaviest kept {worst})",
            outcome.produced.len()
        );
    }
}

fn exit_for(outcome: &Outcome) -> u8 {
    if outcome.complete() {
        EXIT_OK
    } else if outcome.cancelled() {
        EXIT_CANCELLED
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
    if let Some(puzzle_kind) = PuzzleKind::from_tag(kind) {
        return Ok((puzzle_kind.grid_size(), puzzle_kind.regions()));
    }
    // A name the grammar can read describes its own layout, so an
    // invented type needs no entry here (S6b).
    if let Some(composed) = compose_from_tag(kind) {
        return Ok((composed.size, composed.regions));
    }
    bail!(
        "unknown type {kind:?} — use a size like 8, one of \
         4x6x6, 4x8x8, 9x6x6, 8in14, 6in10in14, or a composed name such as \
         4x6x6in16 (n blocks of a×a as <n>x<a>x<a>, centred nesting as <term>in<size>). \
         A layout whose positions are a choice needs --geometry instead"
    )
}

/// The prefix a generated line carries.
///
/// An invented type emits **no** tag, deliberately. The reader resolves
/// a tag through the built-in vocabulary, so emitting `4x10x10:` would
/// produce lines that only a future version could read, and a puzzle
/// file that fails to parse is worse than one that needs its geometry
/// passed alongside. Extending the vocabulary is a decision on the
/// frozen tag format (K28's tag mini-round), not something to slip in
/// with the first invented type; until then `--geometry` is how both
/// halves agree on the regions.
fn emitted_tag(args: &ForgeArgs) -> String {
    match &args.geometry {
        Some(_) => String::new(),
        None => tag_for(&args.kind),
    }
}

/// M25: render a geometry so a mistyped origin is visible instead of
/// mysterious. Takes a file or a built-in name, because someone checking
/// their own file usually wants to compare it against a known one.
fn run_inspect(args: InspectArgs) -> Result<u8> {
    // A composed name describes its own layout, so inspect resolves it
    // the same way forge does. Without this, `forge --kind 4x6x6in16`
    // worked while `inspect 4x6x6in16` said the name did not exist.
    if bpt_forge::geometry::builtin(&args.geometry).is_none()
        && let Some(composed) = compose_from_tag(&args.geometry)
    {
        let geometry = bpt_forge::geometry::Geometry::composed(
            args.geometry.clone(),
            composed.size,
            &composed.regions,
        );
        print!("{}", bpt_forge::inspect::render(&geometry));
        std::io::stdout().flush().ok();
        return Ok(EXIT_OK);
    }
    let geometry = match bpt_forge::geometry::builtin(&args.geometry) {
        Some(result) => result
            .map_err(|e| anyhow::anyhow!("the built-in {} is not usable: {e}", args.geometry))?,
        None => {
            let path = Path::new(&args.geometry);
            if !path.exists() {
                bail!(
                    "{:?} is not a geometry file, a built-in type, or a composed \
                     name — the built-ins are 4x6x6, 4x8x8, 9x6x6, 8in14, \
                     6in10in14, and a composed name looks like 4x6x6in16",
                    args.geometry
                );
            }
            read_geometry(path)?
        }
    };
    print!("{}", bpt_forge::inspect::render(&geometry));
    std::io::stdout().flush().ok();
    Ok(EXIT_OK)
}

/// Read and validate a geometry file. Reading is the CLI's job: the
/// generator crate only ever sees the text (AR1).
fn read_geometry(path: &Path) -> Result<bpt_forge::geometry::Geometry> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("cannot read {} — check the path", path.display()))?;
    bpt_forge::geometry::Geometry::from_toml(&text)
        .map_err(|e| anyhow::anyhow!("{} is not a usable geometry: {e}", path.display()))
}

/// The prefix a generated line carries for `--kind`.
///
/// A composed name is written back verbatim: it is the description that
/// lets a reader rebuild the regions, so dropping it would produce a
/// line that reads as a plain grid.
fn tag_for(kind: &str) -> String {
    if let Some(k) = PuzzleKind::from_tag(kind) {
        return k.tag().map(|t| format!("{t}:")).unwrap_or_default();
    }
    if compose_from_tag(kind).is_some() {
        return format!("{kind}:");
    }
    String::new()
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
