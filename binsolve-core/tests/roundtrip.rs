//! K7 property test: parse → serialize is the identity on any
//! syntactically valid line, for every puzzle kind.

use binsolve_core::parse::{parse_line, serialize};
use proptest::prelude::*;

fn arbitrary_line() -> impl Strategy<Value = String> {
    let standard = (2usize..=9).prop_flat_map(|half| {
        let n = half * 2;
        proptest::collection::vec(prop_oneof![Just('0'), Just('1'), Just('.')], n * n)
            .prop_map(String::from_iter)
    });
    let tagged = prop_oneof![
        Just(("4x6x6", 12usize)),
        Just(("4x8x8", 16)),
        Just(("9x6x6", 18)),
        Just(("8in14", 14)),
        Just(("6in10in14", 14)),
    ]
    .prop_flat_map(|(tag, n)| {
        proptest::collection::vec(prop_oneof![Just('0'), Just('1'), Just('.')], n * n)
            .prop_map(move |cells| format!("{tag}:{}", String::from_iter(cells)))
    });
    prop_oneof![standard, tagged]
}

proptest! {
    #[test]
    fn k7_parse_serialize_identity(line in arbitrary_line()) {
        let puzzle = parse_line(&line).expect("generated line is valid");
        prop_assert_eq!(serialize(&puzzle), line);
    }

    #[test]
    fn k7_arbitrary_bytes_never_panic(line in ".{0,400}") {
        // Precursor of the M7 fuzz target: any string errors or parses.
        let _ = parse_line(&line);
    }
}
