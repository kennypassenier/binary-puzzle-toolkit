//! K4 (a puzzle type is data, validated with remedy-carrying errors) and
//! AR3b (rectangular regions), AR13.4 (rule toggles), AR13.5 (invalid
//! and infeasible are different answers).

use bpt_forge::error::{GeometryError, SizeProblem};
use bpt_forge::geometry::{BUILTIN_TAGS, Geometry, RuleSetSpec, builtin};

#[test]
fn k4_every_builtin_loads_and_validates() {
    for tag in BUILTIN_TAGS {
        let geometry = builtin(tag)
            .unwrap_or_else(|| panic!("{tag} is not a built-in"))
            .unwrap_or_else(|e| panic!("{tag} failed to load: {e}"));
        assert_eq!(geometry.tag.as_deref(), Some(tag));
        assert!(geometry.validate().is_ok());
    }
}

#[test]
fn k4_builtin_shapes_match_their_names() {
    let cases = [
        ("4x6x6", 12, 5),
        ("4x8x8", 16, 5),
        ("9x6x6", 18, 10),
        ("8in14", 14, 2),
        ("6in10in14", 14, 3),
    ];
    for (tag, size, regions) in cases {
        let geometry = builtin(tag).unwrap().unwrap();
        assert_eq!(geometry.size, size, "{tag} grid size");
        assert_eq!(geometry.regions.len(), regions, "{tag} region count");
        // Every composite type constrains the whole grid as well as its
        // parts; that is what makes them harder than their pieces.
        assert!(
            geometry
                .regions
                .iter()
                .any(|r| r.row == 0 && r.col == 0 && r.rows == size && r.cols == size),
            "{tag} has no whole-grid region"
        );
        assert!(geometry.uncovered_cells().is_empty(), "{tag} has holes");
    }
}

#[test]
fn k4_standard_is_one_region_and_rejects_odd_sizes() {
    let ten = Geometry::standard(10).unwrap();
    assert_eq!(ten.regions.len(), 1);
    assert_eq!(ten.tag, None);

    let err = Geometry::standard(9).unwrap_err();
    assert_eq!(
        err,
        GeometryError::GridSize {
            size: 9,
            reason: SizeProblem::Odd
        }
    );
    // Standing rule 11: the message says what to do about it.
    let text = err.to_string();
    assert!(text.contains("Remedy:"), "no remedy in: {text}");
    assert!(
        text.contains('8') && text.contains("10"),
        "no suggestion in: {text}"
    );
}

#[test]
fn k4_out_of_bounds_region_names_the_offending_region() {
    let toml = r#"
size = 10
[[regions]]
row = 0
col = 0
rows = 10
cols = 10
[[regions]]
row = 6
col = 6
rows = 6
cols = 6
"#;
    let err = Geometry::from_toml(toml).unwrap_err();
    match err {
        GeometryError::RegionOutOfBounds { region, size, .. } => {
            assert_eq!(region, 1, "the second region is the broken one");
            assert_eq!(size, 10);
        }
        other => panic!("expected an out-of-bounds error, got {other:?}"),
    }
    let text = err.to_string();
    assert!(
        text.contains("region 1"),
        "the message must name it: {text}"
    );
    assert!(text.contains("Remedy:"), "no remedy in: {text}");
}

#[test]
fn k4_odd_region_side_is_refused_unless_balance_is_off() {
    let with_balance = r#"
size = 10
[[regions]]
row = 0
col = 0
rows = 10
cols = 10
[[regions]]
row = 0
col = 0
rows = 5
cols = 4
"#;
    let err = Geometry::from_toml(with_balance).unwrap_err();
    assert!(matches!(
        err,
        GeometryError::RegionSideOdd {
            region: 1,
            rows: 5,
            cols: 4
        }
    ));

    // AR13.4: a region may opt out of balance, and then its shape is
    // free — the toggle exists so a counter-example is a data fix.
    let without_balance = r#"
size = 10
[[regions]]
row = 0
col = 0
rows = 10
cols = 10
[[regions]]
row = 0
col = 0
rows = 5
cols = 4
[regions.rules]
balance = false
"#;
    let geometry = Geometry::from_toml(without_balance).unwrap();
    assert!(!geometry.regions[1].rules.balance);
    assert!(geometry.regions[1].rules.no_triples, "other rules stay on");
}

#[test]
fn ar3b_rectangular_regions_are_accepted() {
    // The overlapping-band family (scope G3) needs non-square regions;
    // this is the shape binsolve cannot express until mini-round B3.
    // A 6x12 half rather than the 4x12 band this test used to carry:
    // that band is impossible and now says so, because twelve columns of
    // height 4 cannot all differ when only six such lines exist. The
    // claim being tested is that a rectangle is *representable*, which
    // never depended on that particular rectangle being fillable.
    let band = r#"
size = 12
[[regions]]
row = 0
col = 0
rows = 12
cols = 12
[[regions]]
row = 6
col = 0
rows = 6
cols = 12
"#;
    let geometry = Geometry::from_toml(band).unwrap();
    assert_eq!(geometry.regions[1].rows, 6);
    assert_eq!(geometry.regions[1].cols, 12);
}

#[test]
fn k4_no_regions_and_empty_region_are_named_distinctly() {
    let none = Geometry::from_toml("size = 8\nregions = []\n").unwrap_err();
    assert_eq!(none, GeometryError::NoRegions);
    assert!(none.to_string().contains("Remedy:"));

    let empty = r#"
size = 8
[[regions]]
row = 0
col = 0
rows = 0
cols = 8
"#;
    assert_eq!(
        Geometry::from_toml(empty).unwrap_err(),
        GeometryError::RegionEmpty { region: 0 }
    );
}

#[test]
fn k4_syntax_errors_and_unknown_keys_are_reported_as_syntax() {
    let broken = Geometry::from_toml("size = ").unwrap_err();
    assert!(matches!(broken, GeometryError::Syntax { .. }));

    // deny_unknown_fields: a typo'd key is a mistake, not something to
    // silently ignore (standing rule 12, no silent fallbacks).
    let typo = r#"
size = 8
[[regions]]
row = 0
col = 0
rows = 8
colls = 8
"#;
    assert!(matches!(
        Geometry::from_toml(typo).unwrap_err(),
        GeometryError::Syntax { .. }
    ));
}

#[test]
fn ar13_5_infeasible_and_budget_exhausted_are_different_answers() {
    // The distinction K4's tests depend on: "this shape can never work"
    // is a fact about the geometry; "I gave up" is a fact about the run.
    let infeasible = GeometryError::ProvedInfeasible {
        detail: "the inner and outer regions disagree on row 3".into(),
    };
    let gave_up = GeometryError::BudgetExhausted {
        work_units: 500_000,
    };
    assert_ne!(infeasible, gave_up);

    let infeasible_text = infeasible.to_string();
    let gave_up_text = gave_up.to_string();
    assert!(infeasible_text.contains("no solution at all"));
    assert!(
        gave_up_text.contains("NOT proof"),
        "budget exhaustion must not read as impossibility: {gave_up_text}"
    );
    assert!(infeasible_text.contains("Remedy:") && gave_up_text.contains("Remedy:"));
}

#[test]
fn k4_innermost_region_wins_for_nested_types() {
    let geometry = builtin("6in10in14").unwrap().unwrap();
    // Centre cell sits in all three regions; the 6x6 is the smallest.
    let centre = geometry.innermost_at(7, 7).unwrap();
    assert_eq!(geometry.regions[centre].rows, 6);
    assert_eq!(geometry.coverage_at(7, 7), 3);
    // A corner is only in the whole-grid region.
    assert_eq!(geometry.coverage_at(0, 0), 1);
}

#[test]
fn k4_rule_defaults_are_all_on() {
    assert_eq!(RuleSetSpec::default(), RuleSetSpec::ALL);
}

#[test]
fn k4_a_plausible_large_size_is_accepted_not_capped() {
    // Scope G1: sizes beyond the site's 14x14 are in scope, and the
    // practical ceiling comes from measured generation time, never from a
    // constant in the code. A review found an earlier 32-cell cap here.
    for size in [16, 20, 24, 40, 100] {
        assert!(
            Geometry::standard(size).is_ok(),
            "{size} should be representable"
        );
    }
}

#[test]
fn k4_absurd_size_is_refused_before_anything_loops_over_it() {
    // Without a guard this file would send the renderer and the coverage
    // scan over 2^63 rows. The refusal is a resource guard, and its
    // message says so rather than pretending to be a performance ceiling.
    // Even, so it gets past the parity check and reaches the guard.
    let absurd = format!("size = {}\nregions = []\n", u32::MAX - 1);
    let err = Geometry::from_toml(&absurd).unwrap_err();
    let text = err.to_string();
    assert!(text.contains("guard"), "{text}");
    assert!(text.contains("Remedy:"), "{text}");
}

#[test]
fn ar6_error_messages_never_panic_on_absurd_input() {
    // Found by the L1 review: the odd-size message computed size + 1 and
    // overflowed on usize::MAX, so formatting the error crashed instead of
    // reporting it. The core must survive any input, including silly ones.
    for size in [usize::MAX, usize::MAX - 1, 0, 1] {
        let err = GeometryError::GridSize {
            size,
            reason: if size % 2 == 0 {
                SizeProblem::Zero
            } else {
                SizeProblem::Odd
            },
        };
        let text = err.to_string();
        assert!(text.contains("Remedy:"), "size {size}: {text}");
    }
}

/// A region can be too flat to satisfy its own unique-lines rule, and
/// that is decidable without any search: there are only C(h, h/2)
/// balanced lines of length h.
///
/// Found by trying a candidate type — three 4x12 bands in a 12x12 —
/// which needs twelve distinct balanced columns of height 4 where only
/// six exist. Before this check it cost a full search that ended in
/// "no solution, no seed will help".
#[test]
fn k23_a_region_too_flat_for_distinct_lines_is_refused_structurally() {
    let toml = "size = 12\n\n\
        [[regions]]\nrow = 0\ncol = 0\nrows = 12\ncols = 12\n\n\
        [[regions]]\nrow = 0\ncol = 0\nrows = 4\ncols = 12\n";
    let err = bpt_forge::geometry::Geometry::from_toml(toml)
        .expect_err("a 4x12 band cannot keep twelve columns distinct");
    let text = err.to_string();
    assert!(text.contains("only 6 distinct"), "{text}");
    assert!(
        text.contains("Remedy"),
        "every error carries a remedy: {text}"
    );
}

/// The check must not reject shapes that are merely rectangular: two
/// 6x12 halves of a 12x12 need twelve distinct columns of height 6, and
/// twenty exist.
#[test]
fn k23_a_rectangular_region_that_fits_is_accepted() {
    let toml = "size = 12\n\n\
        [[regions]]\nrow = 0\ncol = 0\nrows = 12\ncols = 12\n\n\
        [[regions]]\nrow = 0\ncol = 0\nrows = 6\ncols = 12\n\n\
        [[regions]]\nrow = 6\ncol = 0\nrows = 6\ncols = 12\n";
    bpt_forge::geometry::Geometry::from_toml(toml).expect("6x12 halves are fillable");
}
