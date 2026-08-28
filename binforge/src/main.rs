//! K10: the command-line generator. Owns everything the core may not
//! touch — files, terminal, signals, threads (AR1). Generation arrives
//! in L3-L5; today the binary exposes the geometry inspector (M6).

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use binforge_core::geometry::{BUILTIN_TAGS, Geometry, builtin};
use binforge_core::inspect;
use clap::{Parser, Subcommand};

/// AR11: 0 everything requested was produced, 1 partial, 2 usage or
/// file or geometry error. Stated explicitly because binsolve froze the
/// same three numbers with different meanings.
const EXIT_OK: u8 = 0;
const EXIT_USAGE: u8 = 2;

#[derive(Parser)]
#[command(
    name = "binforge",
    version,
    about = "Generator for binary puzzles (Takuzu/Binairo)"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Render a puzzle geometry as text and report whether it is valid.
    Inspect {
        /// Built-in type tag, e.g. 4x8x8, or a standard size like 10.
        #[arg(value_name = "TAG-OR-SIZE", conflicts_with = "file")]
        target: Option<String>,
        /// Read the geometry from a TOML file instead.
        #[arg(long, value_name = "PATH")]
        file: Option<PathBuf>,
    },
    /// List the built-in puzzle types.
    Types,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            // Errors carry their own remedy (standing rule 11); the chain
            // adds the context of what we were doing when it happened.
            eprintln!("binforge: {error:#}");
            ExitCode::from(EXIT_USAGE)
        }
    }
}

fn run() -> Result<u8> {
    let cli = Cli::parse();
    match cli.command {
        Command::Types => {
            for tag in BUILTIN_TAGS {
                println!("{tag}");
            }
            println!("<even number>  standard n x n, e.g. 10");
            Ok(EXIT_OK)
        }
        Command::Inspect { target, file } => {
            let geometry = load(target, file)?;
            print!("{}", inspect::render(&geometry));
            Ok(EXIT_OK)
        }
    }
}

/// Resolve what the user asked for into a geometry: an explicit file, a
/// built-in tag, or a bare even number meaning a standard grid.
fn load(target: Option<String>, file: Option<PathBuf>) -> Result<Geometry> {
    if let Some(path) = file {
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read geometry file {}", path.display()))?;
        return Geometry::from_toml(&text)
            .with_context(|| format!("{} is not a usable geometry", path.display()));
    }

    let Some(target) = target else {
        bail!(
            "say which geometry to inspect\n\
             Remedy: pass a built-in tag (see `binforge types`), a standard size \
             like 10, or --file <PATH>."
        );
    };

    if let Some(result) = builtin(&target) {
        return result.with_context(|| format!("built-in geometry {target} is broken"));
    }

    if let Ok(size) = target.parse::<usize>() {
        return Geometry::standard(size)
            .with_context(|| format!("{size} is not a usable standard size"));
    }

    bail!(
        "unknown geometry `{target}`\n\
         Remedy: use one of {}, a standard even size like 10, or --file <PATH>.",
        BUILTIN_TAGS.join(", ")
    );
}
