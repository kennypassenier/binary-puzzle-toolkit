//! M7: the parser must never panic on arbitrary input, and whatever it
//! accepts must round-trip through serialization.

#![no_main]

use bpt_core::parse::{parse_line, serialize};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    // Cap the input: a multi-megabyte line is a memory test, not a
    // logic test, and libFuzzer explores length aggressively.
    if text.len() > 4096 {
        return;
    }
    if let Ok(puzzle) = parse_line(text) {
        let round_tripped = serialize(&puzzle);
        let expected = text.strip_suffix('\r').unwrap_or(text);
        assert_eq!(
            round_tripped, expected,
            "accepted input must serialize back identically"
        );
    }
});
