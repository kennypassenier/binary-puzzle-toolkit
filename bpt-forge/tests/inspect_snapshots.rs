//! M6: the geometry inspector's rendering, pinned as snapshots so an
//! accidental change to the map is visible in review rather than silent.

use bpt_forge::geometry::{Geometry, builtin};
use bpt_forge::inspect;

#[test]
fn m6_renders_every_builtin() {
    for tag in bpt_forge::geometry::BUILTIN_TAGS {
        let geometry = builtin(tag).unwrap().unwrap();
        insta::assert_snapshot!(format!("builtin_{tag}"), inspect::render(&geometry));
    }
}

#[test]
fn m6_renders_a_standard_grid() {
    insta::assert_snapshot!(
        "standard_10",
        inspect::render(&Geometry::standard(10).unwrap())
    );
}

#[test]
fn m6_reports_uncovered_cells_with_a_remedy() {
    // The failure an off-by-one origin actually produces: a strip that
    // no region constrains. The map shows it as dots.
    let holed = r#"
size = 8
[[regions]]
row = 0
col = 0
rows = 8
cols = 6
"#;
    let geometry = Geometry::from_toml(holed).unwrap();
    let rendered = inspect::render(&geometry);
    assert!(rendered.contains("covered by no region"), "{rendered}");
    assert!(rendered.contains("Remedy:"), "{rendered}");
    insta::assert_snapshot!("uncovered_strip", rendered);
}

#[test]
fn m6_render_refuses_an_unvalidated_geometry_instead_of_looping() {
    // Geometry has public fields, so an unchecked instance can reach the
    // renderer. It must answer, not loop over a bogus size.
    let bogus = Geometry {
        tag: None,
        size: usize::MAX,
        regions: vec![],
    };
    let rendered = inspect::render(&bogus);
    assert!(rendered.contains("cannot be rendered"), "{rendered}");
    assert!(rendered.contains("Remedy:"), "{rendered}");
}
