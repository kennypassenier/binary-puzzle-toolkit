//! AR7 regression vectors: the frozen format, pinned as files [K7].

use bpt_core::parse::{ParseError, parse_line, serialize};

#[test]
fn k7_every_valid_vector_parses_and_round_trips() {
    let content = include_str!("fixtures/format/valid.txt");
    for (i, line) in content.lines().enumerate() {
        let puzzle = parse_line(line).unwrap_or_else(|e| panic!("valid.txt line {}: {e}", i + 1));
        assert_eq!(
            serialize(&puzzle),
            line,
            "valid.txt line {} does not round-trip",
            i + 1
        );
    }
}

#[test]
fn k7_every_invalid_vector_fails_with_the_expected_class() {
    let content = include_str!("fixtures/format/invalid.txt");
    for (i, entry) in content.lines().enumerate() {
        let (expected, line) = entry
            .split_once('|')
            .unwrap_or_else(|| panic!("invalid.txt line {} lacks 'class|line'", i + 1));
        let err = parse_line(line)
            .err()
            .unwrap_or_else(|| panic!("invalid.txt line {} unexpectedly parsed", i + 1));
        let class = match err {
            ParseError::EmptyLine => "empty",
            ParseError::UnknownTag { .. } => "unknown-tag",
            ParseError::InvalidChar { .. } => "invalid-char",
            ParseError::TagLengthMismatch { .. } => "tag-length",
            ParseError::NotSquare { .. } => "not-square",
            ParseError::OddSize { .. } => "odd-size",
            other => panic!("invalid.txt line {}: unexpected error {other:?}", i + 1),
        };
        assert_eq!(class, expected, "invalid.txt line {}", i + 1);
    }
}
