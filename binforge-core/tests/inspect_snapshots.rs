//! M6: the geometry inspector's rendering, pinned as snapshots so an
//! accidental change to the map is visible in review rather than silent.

use binforge_core::geometry::{Geometry, builtin};
use binforge_core::inspect;

#[test]
fn m6_renders_every_builtin() {
    for tag in binforge_core::geometry::BUILTIN_TAGS {
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
