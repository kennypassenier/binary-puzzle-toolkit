//! M6: render a geometry as text so a mistyped origin is visible instead
//! of mysterious. Pure string building — printing is the CLI's job (AR1).

use crate::geometry::Geometry;

/// Letters label regions in the map; beyond 26 regions the label repeats,
/// which is legible enough for a debugging aid and keeps the grid aligned.
fn label(index: usize) -> char {
    (b'A' + (index % 26) as u8) as char
}

/// A map of the grid where each cell shows the innermost region covering
/// it, followed by a legend and a verdict. Every region gets a line, so
/// an origin that is off by one shows up as a shifted block.
pub fn render(geometry: &Geometry) -> String {
    // `Geometry` has public fields, so an instance can reach here without
    // ever passing `validate()`. Rendering an unchecked one would loop over
    // whatever `size` claims to be; refusing with the remedy is the honest
    // answer and keeps the promise that nothing hangs (AR10).
    if let Err(error) = geometry.validate() {
        return format!("this geometry cannot be rendered: {error}\n");
    }

    let mut out = String::new();

    let tag = geometry.tag.as_deref().unwrap_or("(standard, untagged)");
    out.push_str(&format!(
        "{tag} — {size}x{size}, {count} region(s)\n\n",
        size = geometry.size,
        count = geometry.regions.len()
    ));

    // Column ruler in tens and units, so a 14-wide grid stays countable.
    let indent = "    ";
    if geometry.size > 9 {
        out.push_str(indent);
        for col in 0..geometry.size {
            out.push(if col >= 10 {
                char::from_digit((col / 10) as u32, 10).unwrap_or('?')
            } else {
                ' '
            });
        }
        out.push('\n');
    }
    out.push_str(indent);
    for col in 0..geometry.size {
        out.push(char::from_digit((col % 10) as u32, 10).unwrap_or('?'));
    }
    out.push('\n');

    for row in 0..geometry.size {
        out.push_str(&format!("{row:>3} "));
        for col in 0..geometry.size {
            match geometry.innermost_at(row, col) {
                Some(index) => out.push(label(index)),
                // A cell no region constrains: almost always a typo.
                None => out.push('.'),
            }
        }
        out.push('\n');
    }

    out.push_str("\nRegions (innermost wins in the map above):\n");
    for (index, region) in geometry.regions.iter().enumerate() {
        let mut rules = Vec::new();
        if region.rules.balance {
            rules.push("balance");
        }
        if region.rules.no_triples {
            rules.push("no-triples");
        }
        if region.rules.unique_lines {
            rules.push("unique-lines");
        }
        out.push_str(&format!(
            "  {} at ({},{}) {}x{} [{}]\n",
            label(index),
            region.row,
            region.col,
            region.rows,
            region.cols,
            rules.join(", ")
        ));
    }

    let uncovered = geometry.uncovered_cells();
    out.push_str("\nVerdict: structurally valid");
    if uncovered.is_empty() {
        out.push_str(", every cell constrained.\n");
    } else {
        out.push_str(&format!(
            ", but {} cell(s) are covered by no region, starting at ({},{}).\n\
             Remedy: check the origins — an off-by-one leaves a strip unconstrained.\n",
            uncovered.len(),
            uncovered[0].0,
            uncovered[0].1
        ));
    }
    out.push_str(
        "Feasibility is not checked here: proving a geometry solvable needs the solver (L3).\n",
    );
    out
}
