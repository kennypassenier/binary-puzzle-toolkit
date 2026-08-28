//! The AR7 text format: `[tag ":"] grid`, one puzzle per line, plus the
//! AR12 two-line corpus format. Errors carry their remedy (rule 11).
//! Hand-rolled Error impls per the T5 amendment (core stays dep-free).

use crate::grid::{Cell, Grid};
use crate::region::{Puzzle, PuzzleKind};
use std::error::Error;
use std::fmt;

pub const KNOWN_TAGS: [&str; 5] = ["4x6x6", "4x8x8", "9x6x6", "8in14", "6in10in14"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    EmptyLine,
    UnknownTag {
        tag: String,
    },
    InvalidChar {
        position: usize,
        found: char,
    },
    TagLengthMismatch {
        tag: &'static str,
        expected: usize,
        got: usize,
    },
    NotSquare {
        got: usize,
    },
    OddSize {
        n: usize,
    },
    BadSolutionLine {
        found: String,
    },
    SolutionLengthMismatch {
        expected: usize,
        got: usize,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::EmptyLine => {
                write!(
                    f,
                    "line is empty — provide a puzzle like 1..0.0... or remove the line"
                )
            }
            ParseError::UnknownTag { tag } => write!(
                f,
                "unknown puzzle type tag '{tag}' — use one of {}, or no tag for a standard square puzzle",
                KNOWN_TAGS.join(", ")
            ),
            ParseError::InvalidChar { position, found } => write!(
                f,
                "invalid character {found:?} at position {position} — a puzzle contains only '0', '1' and '.'"
            ),
            ParseError::TagLengthMismatch { tag, expected, got } => write!(
                f,
                "tag '{tag}' requires exactly {expected} cells but the grid has {got} — fix the grid or the tag"
            ),
            ParseError::NotSquare { got } => write!(
                f,
                "grid has {got} cells, which is not a square of an even size — a standard puzzle needs n*n cells with n even (36, 64, 100, ...)"
            ),
            ParseError::OddSize { n } => write!(
                f,
                "grid is {n}x{n}, but binary puzzles need an even side length — the balance rule cannot hold on odd lines"
            ),
            ParseError::BadSolutionLine { found } => write!(
                f,
                "second corpus line must start with 'solution:' — found {found:?}"
            ),
            ParseError::SolutionLengthMismatch { expected, got } => write!(
                f,
                "solution has {got} cells but the puzzle has {expected} — both lines must describe the same grid"
            ),
        }
    }
}

impl Error for ParseError {}

/// Parse a grid character sequence; positions in errors are 0-based
/// offsets within `s`.
pub fn parse_grid_chars(s: &str) -> Result<Vec<Cell>, ParseError> {
    s.chars()
        .enumerate()
        .map(|(position, ch)| {
            Cell::from_char(ch).ok_or(ParseError::InvalidChar {
                position,
                found: ch,
            })
        })
        .collect()
}

/// Strip the line ending tolerance AR7 grants: trailing `\r` (CRLF
/// input from Windows tools) is accepted everywhere.
fn strip_cr(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
}

/// Parse one AR7 line into a puzzle.
pub fn parse_line(line: &str) -> Result<Puzzle, ParseError> {
    let line = strip_cr(line);
    if line.is_empty() {
        return Err(ParseError::EmptyLine);
    }
    let (kind, grid_part) = match line.split_once(':') {
        Some((tag, rest)) => {
            let kind = PuzzleKind::from_tag(tag).ok_or_else(|| ParseError::UnknownTag {
                tag: tag.to_string(),
            })?;
            (Some(kind), rest)
        }
        None => (None, line),
    };
    let cells = parse_grid_chars(grid_part)?;
    let kind = match kind {
        Some(kind) => {
            let expected = kind.grid_size() * kind.grid_size();
            if cells.len() != expected {
                return Err(ParseError::TagLengthMismatch {
                    tag: kind.tag().expect("tagged kinds have a tag"),
                    expected,
                    got: cells.len(),
                });
            }
            kind
        }
        None => {
            let n = (cells.len() as f64).sqrt() as usize;
            // Probe n-1, n, n+1 rather than trusting the cast. Measured
            // over every perfect square up to 4096 the naive conversion
            // is never wrong, so this is defensive rather than load
            // bearing — it keeps the intent explicit and costs nothing.
            let n = [n.saturating_sub(1), n, n + 1]
                .into_iter()
                .find(|k| k * k == cells.len())
                .ok_or(ParseError::NotSquare { got: cells.len() })?;
            if n == 0 || n % 2 != 0 {
                if n == 0 || n * n != cells.len() {
                    return Err(ParseError::NotSquare { got: cells.len() });
                }
                return Err(ParseError::OddSize { n });
            }
            PuzzleKind::Standard { n }
        }
    };
    let n = kind.grid_size();
    Ok(Puzzle {
        kind,
        givens: Grid::from_cells(n, cells),
        // Text always describes one of the six known kinds; a custom
        // geometry reaches the solver through Puzzle::custom instead.
        custom_regions: None,
    })
}

/// Canonical serialization: parse(serialize(p)) == p, and for canonical
/// input serialize(parse(line)) == line (round-trip property, K7).
pub fn serialize(puzzle: &Puzzle) -> String {
    match puzzle.kind.tag() {
        Some(tag) => format!("{tag}:{}", puzzle.givens.to_line()),
        None => puzzle.givens.to_line(),
    }
}

/// Parse an AR12 corpus file: line 1 = AR7 puzzle line, line 2
/// (optional) = `solution:<grid>`. Later lines are ignored by design —
/// the corpus format owns the file, the AR7 grammar owns line 1 only.
pub fn parse_corpus_file(content: &str) -> Result<(Puzzle, Option<Grid>), ParseError> {
    let mut lines = content.lines();
    let puzzle = parse_line(lines.next().unwrap_or(""))?;
    let solution = match lines.next().map(strip_cr) {
        None | Some("") => None,
        Some(second) => {
            let grid_part =
                second
                    .strip_prefix("solution:")
                    .ok_or_else(|| ParseError::BadSolutionLine {
                        found: second.chars().take(20).collect(),
                    })?;
            let cells = parse_grid_chars(grid_part)?;
            let n = puzzle.givens.size();
            if cells.len() != n * n {
                return Err(ParseError::SolutionLengthMismatch {
                    expected: n * n,
                    got: cells.len(),
                });
            }
            Some(Grid::from_cells(n, cells))
        }
    };
    Ok((puzzle, solution))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k7_untagged_size_inference() {
        let p = parse_line(&".".repeat(36)).unwrap();
        assert_eq!(p.kind, PuzzleKind::Standard { n: 6 });
    }

    #[test]
    fn k7_tagged_parse_and_round_trip() {
        let line = format!("8in14:{}", ".".repeat(196));
        let p = parse_line(&line).unwrap();
        assert_eq!(p.kind, PuzzleKind::EightIn14);
        assert_eq!(serialize(&p), line);
    }

    #[test]
    fn k7_crlf_tolerated() {
        let line = format!("{}\r", ".".repeat(16));
        let p = parse_line(&line).unwrap();
        assert_eq!(p.kind, PuzzleKind::Standard { n: 4 });
    }

    #[test]
    fn k7_error_classes_carry_remedies() {
        let cases: Vec<(String, &str)> = vec![
            (String::new(), "line is empty"),
            (
                format!("7x7x7:{}", ".".repeat(36)),
                "unknown puzzle type tag",
            ),
            ("1..0x0..1".to_string(), "invalid character"),
            (format!("4x8x8:{}", ".".repeat(64)), "requires exactly 256"),
            (".".repeat(35), "not a square"),
            (".".repeat(25), "even side length"),
        ];
        for (line, expected) in cases {
            let err = parse_line(&line).unwrap_err().to_string();
            assert!(err.contains(expected), "{line:?} -> {err}");
        }
    }

    #[test]
    fn k7_multibyte_input_is_error_not_panic() {
        // M7's first prey: multi-byte chars must not slice-panic.
        let err = parse_line("1..0é0...").unwrap_err();
        assert!(matches!(err, ParseError::InvalidChar { .. }));
    }

    #[test]
    fn m4_corpus_file_with_solution() {
        let content = format!(
            "{}\nsolution:{}\n",
            ".1..".to_owned() + &".".repeat(12),
            "0110100101101001"
        );
        let (p, sol) = parse_corpus_file(&content).unwrap();
        assert_eq!(p.givens.size(), 4);
        assert!(sol.unwrap().is_complete());
    }

    #[test]
    fn m4_corpus_bad_solution_prefix() {
        let content = format!("{}\nanswer:0110\n", ".".repeat(16));
        assert!(matches!(
            parse_corpus_file(&content),
            Err(ParseError::BadSolutionLine { .. })
        ));
    }
}
