//! Regions and rule sets: the AR3 model. A puzzle is one grid plus a
//! list of overlapping square regions; every rule is checked per
//! region, never on the raw grid.

use crate::grid::{Cell, Grid};
use std::fmt;

/// Which of the three rule families a region enforces. All-on for every
/// known puzzle type (empirically verified against published solutions,
/// 2026-08-12); the mask exists so a counter-example is a data fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleSet {
    pub balance: bool,
    pub no_triples: bool,
    pub unique_lines: bool,
}

impl RuleSet {
    pub const ALL: RuleSet = RuleSet {
        balance: true,
        no_triples: true,
        unique_lines: true,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub row: usize,
    pub col: usize,
    pub rows: usize,
    pub cols: usize,
    pub rules: RuleSet,
}

impl Region {
    /// A square region enforcing every rule — what all six known puzzle
    /// kinds are made of. Regions are rectangular in general so that a
    /// generated geometry can define non-square areas; every rule below
    /// reads `rows`/`cols` rather than assuming they are equal.
    fn all(row: usize, col: usize, n: usize) -> Self {
        Region::square(row, col, n)
    }

    pub fn square(row: usize, col: usize, n: usize) -> Self {
        Region {
            row,
            col,
            rows: n,
            cols: n,
            rules: RuleSet::ALL,
        }
    }

    /// Number of cells in one line of this region: a row spans `cols`
    /// cells, a column spans `rows`.
    pub fn line_len(&self, is_row: bool) -> usize {
        if is_row { self.cols } else { self.rows }
    }

    /// How many lines this region has in the given direction.
    pub fn line_count(&self, is_row: bool) -> usize {
        if is_row { self.rows } else { self.cols }
    }

    pub fn contains(&self, row: usize, col: usize) -> bool {
        row >= self.row
            && row < self.row + self.rows
            && col >= self.col
            && col < self.col + self.cols
    }

    /// Cells of one line of this region, in grid coordinates.
    pub fn line_cells(&self, grid: &Grid, is_row: bool, index: usize) -> Vec<Cell> {
        (0..self.line_len(is_row))
            .map(|i| {
                if is_row {
                    grid.get(self.row + index, self.col + i)
                } else {
                    grid.get(self.row + i, self.col + index)
                }
            })
            .collect()
    }
}

/// The five special types (K2a–K2e) plus standard n×n.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PuzzleKind {
    Standard {
        n: usize,
    },
    FourTimes6x6,
    FourTimes8x8,
    NineTimes6x6,
    EightIn14,
    SixIn10In14,
    /// A geometry the generator invented: any grid size with any set of
    /// regions. The solver treats it exactly like a known kind — the
    /// rules were never per-type, only per-region — so a generated type
    /// can be solved and proven unique without teaching the solver about
    /// it first.
    Custom {
        n: usize,
    },
}

/// A composite type whose whole layout follows from its name (S6b).
///
/// The five published types are not arbitrary labels — they are
/// descriptions, and always were. `4x6x6` says "four 6x6 blocks";
/// `6in10in14` says "a 6x6 inside a 10x10 inside a 14x14". Reading them
/// that way means an invented type that follows the same naming needs no
/// entry anywhere: `4x6x6in16` is a four-quadrant 12x12 centred in a
/// 16x16, and it works the moment someone writes it.
///
/// Two shapes, composable:
/// - `<n>x<a>x<a>` — n blocks of a×a laid out √n by √n.
/// - `<term>in<size>` — the term centred in a larger square; chainable.
///
/// Deliberately no positions. A type whose parts could sit anywhere —
/// two blocks side by side, two that overlap — cannot be described by a
/// name without turning the name into a file format. Those travel as a
/// geometry file instead, which both halves already accept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposedKind {
    pub size: usize,
    pub regions: Vec<Region>,
}

/// Read a tag as a description. `None` means the name does not follow
/// the grammar — never "assume a plain grid", which would silently solve
/// a composite as something easier.
pub fn compose_from_tag(tag: &str) -> Option<ComposedKind> {
    let mut terms = tag.split("in");
    let inner = terms.next()?;
    let (mut size, mut regions) = compose_term(inner)?;
    for outer in terms {
        let outer: usize = outer.parse().ok()?;
        // Centred, and the offset has to be whole: a 6 in a 9 has no
        // centre, and a name that asks for one is not a type.
        if outer <= size || !outer.is_multiple_of(2) || !(outer - size).is_multiple_of(2) {
            return None;
        }
        let offset = (outer - size) / 2;
        for region in &mut regions {
            region.row += offset;
            region.col += offset;
        }
        regions.push(Region::all(0, 0, outer));
        size = outer;
    }
    // A bare size is a plain grid, which needs no tag and must not get
    // one: `12:` and an untagged 12x12 line would be two spellings of
    // the same puzzle.
    if regions.len() < 2 {
        return None;
    }
    Some(ComposedKind { size, regions })
}

/// One term: either `<n>x<a>x<a>` or a bare size.
fn compose_term(term: &str) -> Option<(usize, Vec<Region>)> {
    let mut parts = term.split('x');
    let first: usize = parts.next()?.parse().ok()?;
    match (parts.next(), parts.next(), parts.next()) {
        (None, None, None) => {
            if first == 0 || !first.is_multiple_of(2) {
                return None;
            }
            Some((first, vec![Region::all(0, 0, first)]))
        }
        (Some(a), Some(b), None) => {
            let (a, b): (usize, usize) = (a.parse().ok()?, b.parse().ok()?);
            // Square blocks only, an even side, and a count that really
            // is a square number — `5x6x6` describes no layout.
            let side = first.isqrt();
            if a != b || a == 0 || !a.is_multiple_of(2) || side * side != first {
                return None;
            }
            Some((side * a, tiled(side, a)))
        }
        _ => None,
    }
}

impl PuzzleKind {
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "4x6x6" => Some(PuzzleKind::FourTimes6x6),
            "4x8x8" => Some(PuzzleKind::FourTimes8x8),
            "9x6x6" => Some(PuzzleKind::NineTimes6x6),
            "8in14" => Some(PuzzleKind::EightIn14),
            "6in10in14" => Some(PuzzleKind::SixIn10In14),
            _ => None,
        }
    }

    pub fn tag(&self) -> Option<&'static str> {
        match self {
            // A custom geometry has no reserved tag: it travels with its
            // region list rather than by name.
            PuzzleKind::Standard { .. } | PuzzleKind::Custom { .. } => None,
            PuzzleKind::FourTimes6x6 => Some("4x6x6"),
            PuzzleKind::FourTimes8x8 => Some("4x8x8"),
            PuzzleKind::NineTimes6x6 => Some("9x6x6"),
            PuzzleKind::EightIn14 => Some("8in14"),
            PuzzleKind::SixIn10In14 => Some("6in10in14"),
        }
    }

    pub fn grid_size(&self) -> usize {
        match self {
            PuzzleKind::Standard { n } | PuzzleKind::Custom { n } => *n,
            PuzzleKind::FourTimes6x6 => 12,
            PuzzleKind::FourTimes8x8 => 16,
            PuzzleKind::NineTimes6x6 => 18,
            PuzzleKind::EightIn14 | PuzzleKind::SixIn10In14 => 14,
        }
    }

    /// The AR3 region decomposition. Tiled types list their blocks plus
    /// the whole grid; nested types list the concentric grids.
    pub fn regions(&self) -> Vec<Region> {
        match self {
            PuzzleKind::Standard { n } => vec![Region::all(0, 0, *n)],
            // Custom kinds carry their regions on the Puzzle, not here.
            PuzzleKind::Custom { n } => vec![Region::all(0, 0, *n)],
            PuzzleKind::FourTimes6x6 => tiled(2, 6),
            PuzzleKind::FourTimes8x8 => tiled(2, 8),
            PuzzleKind::NineTimes6x6 => tiled(3, 6),
            PuzzleKind::EightIn14 => {
                vec![Region::all(3, 3, 8), Region::all(0, 0, 14)]
            }
            PuzzleKind::SixIn10In14 => {
                vec![
                    Region::all(4, 4, 6),
                    Region::all(2, 2, 10),
                    Region::all(0, 0, 14),
                ]
            }
        }
    }
}

fn tiled(blocks_per_side: usize, block_n: usize) -> Vec<Region> {
    let whole = blocks_per_side * block_n;
    let mut regions = Vec::with_capacity(blocks_per_side * blocks_per_side + 1);
    for br in 0..blocks_per_side {
        for bc in 0..blocks_per_side {
            regions.push(Region::all(br * block_n, bc * block_n, block_n));
        }
    }
    regions.push(Region::all(0, 0, whole));
    regions
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    pub kind: PuzzleKind,
    pub givens: Grid,
    /// Set only for a custom geometry; the six known kinds derive their
    /// regions from `kind` so that a puzzle read from text can never
    /// disagree with its tag.
    pub custom_regions: Option<Vec<Region>>,
    /// The tag a composed type was read from, kept so it can be written
    /// back. A tag that survives the round trip is the whole point: a
    /// generated `4x6x6in16` line has to read as the same puzzle it was
    /// written from.
    pub composed_tag: Option<String>,
}

impl Puzzle {
    pub fn regions(&self) -> Vec<Region> {
        match &self.custom_regions {
            Some(regions) => regions.clone(),
            None => self.kind.regions(),
        }
    }

    /// A puzzle on an invented geometry: the generator hands over the
    /// regions it built and validated.
    pub fn custom(givens: Grid, regions: Vec<Region>) -> Self {
        Puzzle {
            kind: PuzzleKind::Custom { n: givens.size() },
            givens,
            custom_regions: Some(regions),
            composed_tag: None,
        }
    }

    /// A custom geometry that came from a tag, and can be written back
    /// as one.
    pub fn composed(givens: Grid, regions: Vec<Region>, tag: String) -> Self {
        Puzzle {
            composed_tag: Some(tag),
            ..Puzzle::custom(givens, regions)
        }
    }

    /// The tag this puzzle should be written with, if any.
    pub fn write_tag(&self) -> Option<&str> {
        self.kind.tag().or(self.composed_tag.as_deref())
    }
}

/// One concrete rule violation, locatable and explainable (K6, M3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    Incomplete {
        empty_cells: usize,
    },
    Imbalance {
        region: Region,
        is_row: bool,
        index: usize,
        zeros: usize,
        ones: usize,
    },
    Triple {
        region: Region,
        is_row: bool,
        index: usize,
        start: usize,
        value: Cell,
    },
    DuplicateLines {
        region: Region,
        is_row: bool,
        first: usize,
        second: usize,
    },
    /// More copies of a value than a line may ever hold (detectable on
    /// a partially filled grid).
    TooMany {
        region: Region,
        is_row: bool,
        index: usize,
        value: Cell,
        count: usize,
        max: usize,
    },
    GivenChanged {
        row: usize,
        col: usize,
        given: Cell,
        found: Cell,
    },
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn line_name(is_row: bool, region: &Region, index: usize) -> String {
            let kind = if is_row { "row" } else { "column" };
            let abs = if is_row {
                region.row + index
            } else {
                region.col + index
            };
            if region.row == 0 && region.col == 0 {
                format!("{kind} {abs}")
            } else {
                format!(
                    "{kind} {abs} (within the {rows}x{cols} region at r{r},c{c})",
                    rows = region.rows,
                    cols = region.cols,
                    r = region.row,
                    c = region.col
                )
            }
        }
        match self {
            Violation::Incomplete { empty_cells } => write!(
                f,
                "grid is not completely filled: {empty_cells} empty cell(s) remain"
            ),
            Violation::Imbalance {
                region,
                is_row,
                index,
                zeros,
                ones,
            } => write!(
                f,
                "{} has {zeros} zeros and {ones} ones; each line needs exactly {} of each",
                line_name(*is_row, region, *index),
                region.line_len(*is_row) / 2
            ),
            Violation::Triple {
                region,
                is_row,
                index,
                start,
                value,
            } => write!(
                f,
                "{} has three consecutive {}s starting at position {start}; at most two may touch",
                line_name(*is_row, region, *index),
                value.to_char()
            ),
            Violation::DuplicateLines {
                region,
                is_row,
                first,
                second,
            } => {
                let kind = if *is_row { "rows" } else { "columns" };
                write!(
                    f,
                    "{kind} {} and {} of the {rows}x{cols} region at r{r},c{c} are identical; every line must be unique",
                    first,
                    second,
                    rows = region.rows,
                    cols = region.cols,
                    r = region.row,
                    c = region.col
                )
            }
            Violation::TooMany {
                region,
                is_row,
                index,
                value,
                count,
                max,
            } => write!(
                f,
                "{} already has {count} {}s but may hold at most {max}",
                line_name(*is_row, region, *index),
                value.to_char()
            ),
            Violation::GivenChanged {
                row,
                col,
                given,
                found,
            } => write!(
                f,
                "cell r{row},c{col} was given as {} but the solution has {}; givens may not change",
                given.to_char(),
                found.to_char()
            ),
        }
    }
}

/// Check a completely filled grid against every region rule (M3 core).
/// Returns every violation found, empty = valid.
pub fn validate_solution(grid: &Grid, regions: &[Region]) -> Vec<Violation> {
    let mut violations = Vec::new();
    let empty = grid.cells().iter().filter(|c| c.is_empty()).count();
    if empty > 0 {
        violations.push(Violation::Incomplete { empty_cells: empty });
        return violations;
    }
    for region in regions {
        for is_row in [true, false] {
            let lines: Vec<Vec<Cell>> = (0..region.line_count(is_row))
                .map(|i| region.line_cells(grid, is_row, i))
                .collect();
            for (index, line) in lines.iter().enumerate() {
                if region.rules.balance {
                    let zeros = line.iter().filter(|c| **c == Cell::Zero).count();
                    let ones = line.len() - zeros;
                    if zeros != ones {
                        violations.push(Violation::Imbalance {
                            region: *region,
                            is_row,
                            index,
                            zeros,
                            ones,
                        });
                    }
                }
                if region.rules.no_triples {
                    for start in 0..line.len().saturating_sub(2) {
                        if line[start] == line[start + 1] && line[start] == line[start + 2] {
                            violations.push(Violation::Triple {
                                region: *region,
                                is_row,
                                index,
                                start,
                                value: line[start],
                            });
                        }
                    }
                }
            }
            if region.rules.unique_lines {
                for first in 0..lines.len() {
                    for second in (first + 1)..lines.len() {
                        if lines[first] == lines[second] {
                            violations.push(Violation::DuplicateLines {
                                region: *region,
                                is_row,
                                first,
                                second,
                            });
                        }
                    }
                }
            }
        }
    }
    violations
}

/// Rule violations detectable on a PARTIALLY filled grid (K6): filled
/// triples, over-count of one value, and duplicated completed lines.
/// Empty = nothing provably wrong yet.
pub fn validate_partial(grid: &Grid, regions: &[Region]) -> Vec<Violation> {
    let mut violations = Vec::new();
    for region in regions {
        for is_row in [true, false] {
            let lines: Vec<Vec<Cell>> = (0..region.line_count(is_row))
                .map(|i| region.line_cells(grid, is_row, i))
                .collect();
            for (index, line) in lines.iter().enumerate() {
                if region.rules.balance {
                    for value in [Cell::Zero, Cell::One] {
                        let count = line.iter().filter(|c| **c == value).count();
                        if count > region.line_len(is_row) / 2 {
                            violations.push(Violation::TooMany {
                                region: *region,
                                is_row,
                                index,
                                value,
                                count,
                                max: region.line_len(is_row) / 2,
                            });
                        }
                    }
                }
                if region.rules.no_triples {
                    for start in 0..line.len().saturating_sub(2) {
                        if !line[start].is_empty()
                            && line[start] == line[start + 1]
                            && line[start] == line[start + 2]
                        {
                            violations.push(Violation::Triple {
                                region: *region,
                                is_row,
                                index,
                                start,
                                value: line[start],
                            });
                        }
                    }
                }
            }
            if region.rules.unique_lines {
                for first in 0..lines.len() {
                    if lines[first].iter().any(|c| c.is_empty()) {
                        continue;
                    }
                    for second in (first + 1)..lines.len() {
                        if lines[first] == lines[second] {
                            violations.push(Violation::DuplicateLines {
                                region: *region,
                                is_row,
                                first,
                                second,
                            });
                        }
                    }
                }
            }
        }
    }
    violations
}

/// Check that a solution preserves every given (M3 core).
pub fn validate_givens(givens: &Grid, solution: &Grid) -> Vec<Violation> {
    let n = givens.size();
    let mut violations = Vec::new();
    for r in 0..n {
        for c in 0..n {
            let g = givens.get(r, c);
            if !g.is_empty() && g != solution.get(r, c) {
                violations.push(Violation::GivenChanged {
                    row: r,
                    col: c,
                    given: g,
                    found: solution.get(r, c),
                });
            }
        }
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_grid_chars;

    fn grid(s: &str, n: usize) -> Grid {
        Grid::from_cells(n, parse_grid_chars(s).unwrap())
    }

    #[test]
    fn k2_region_decompositions_have_expected_shapes() {
        assert_eq!(PuzzleKind::Standard { n: 8 }.regions().len(), 1);
        assert_eq!(PuzzleKind::FourTimes6x6.regions().len(), 5);
        assert_eq!(PuzzleKind::FourTimes8x8.regions().len(), 5);
        assert_eq!(PuzzleKind::NineTimes6x6.regions().len(), 10);
        assert_eq!(PuzzleKind::EightIn14.regions().len(), 2);
        assert_eq!(PuzzleKind::SixIn10In14.regions().len(), 3);
        // The nested 6x6 sits centered: offset (14-6)/2 = 4.
        let inner = PuzzleKind::SixIn10In14.regions()[0];
        assert_eq!((inner.row, inner.col, inner.rows, inner.cols), (4, 4, 6, 6));
    }

    #[test]
    fn m3_valid_4x4_passes() {
        // Rows 0011/1100/0101/1010: unique, balanced, triple-free —
        // and so are the columns.
        let g = grid("0011110001011010", 4);
        let regions = PuzzleKind::Standard { n: 4 }.regions();
        assert!(validate_solution(&g, &regions).is_empty());
    }

    #[test]
    fn m3_k6_imbalance_named() {
        let g = grid("0110100101100110", 4);
        let regions = PuzzleKind::Standard { n: 4 }.regions();
        let v = validate_solution(&g, &regions);
        assert!(!v.is_empty());
        let text = v
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("identical") || text.contains("zeros"),
            "{text}"
        );
    }

    #[test]
    fn m3_k6_triple_detected_and_named() {
        // Row 0 of a 4x4 with three zeros: 0001 — triple at start.
        let g = grid("0001111000010111", 4);
        let regions = PuzzleKind::Standard { n: 4 }.regions();
        let v = validate_solution(&g, &regions);
        assert!(v.iter().any(|x| matches!(x, Violation::Triple { .. })));
        let text = v
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("three consecutive"), "{text}");
    }

    #[test]
    fn m3_given_changed_detected() {
        let givens = grid("0...............", 4);
        let solution = grid("1010010110100101", 4);
        let v = validate_givens(&givens, &solution);
        assert_eq!(v.len(), 1);
        assert!(v[0].to_string().contains("givens may not change"));
    }

    #[test]
    fn m3_incomplete_detected() {
        let g = grid("0110.00101101001", 4);
        let regions = PuzzleKind::Standard { n: 4 }.regions();
        let v = validate_solution(&g, &regions);
        assert!(matches!(v[0], Violation::Incomplete { empty_cells: 1 }));
    }
}
