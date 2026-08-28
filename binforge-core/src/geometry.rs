//! K4: a puzzle type is data — a grid size plus the rectangular regions
//! that must each satisfy the Takuzu rules. Built-in types and invented
//! ones are entries in the same model, so a new type is a TOML file
//! rather than a code change here.
//!
//! Regions are rectangular (AR3b) although every type known today is
//! square: the alternative was migrating hand-written geometry files
//! later, and overlapping-band compositions are the one genuinely novel
//! family in scope (G3).

use crate::error::{GeometryError, SizeProblem};
use serde::Deserialize;

/// A resource guard against absurd input, NOT the practical size ceiling:
/// scope G1 puts that ceiling in the G8 performance measurements and says
/// it is never hardcoded. Without any bound a file saying
/// `size = 9223372036854775806` would send the renderer and the coverage
/// scan into a loop over 2^63 rows, so the guard sits far above any
/// plausible puzzle (256x256 = 65 536 cells) and refuses only nonsense.
pub const MAX_SUPPORTED_SIZE: usize = 256;

/// Which Takuzu rules a region enforces. All three hold for every type
/// known today; the toggles exist so a future counter-example is a data
/// fix rather than a code change (AR13.4, mirroring binsolve's RuleSet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSet {
    /// Equal counts of 0 and 1 in every line of the region.
    #[serde(default = "yes")]
    pub balance: bool,
    /// No three identical digits adjacent in a line of the region.
    #[serde(default = "yes")]
    pub no_triples: bool,
    /// No two identical rows, and no two identical columns.
    #[serde(default = "yes")]
    pub unique_lines: bool,
}

fn yes() -> bool {
    true
}

impl Default for RuleSet {
    fn default() -> Self {
        RuleSet {
            balance: true,
            no_triples: true,
            unique_lines: true,
        }
    }
}

impl RuleSet {
    pub const ALL: RuleSet = RuleSet {
        balance: true,
        no_triples: true,
        unique_lines: true,
    };
}

/// One rectangular area of the grid that must satisfy `rules` on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    pub row: usize,
    pub col: usize,
    pub rows: usize,
    pub cols: usize,
    #[serde(default)]
    pub rules: RuleSet,
}

impl Region {
    pub fn square(row: usize, col: usize, n: usize) -> Self {
        Region {
            row,
            col,
            rows: n,
            cols: n,
            rules: RuleSet::ALL,
        }
    }

    pub fn area(&self) -> usize {
        self.rows * self.cols
    }

    pub fn contains(&self, row: usize, col: usize) -> bool {
        row >= self.row
            && row < self.row + self.rows
            && col >= self.col
            && col < self.col + self.cols
    }
}

/// A puzzle type: the grid it lives on plus every region that constrains
/// it. A plain n×n puzzle is one region covering the whole grid; the
/// composite types add sub-grids on top of that.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Geometry {
    /// The tag that identifies this type in the puzzle line format, e.g.
    /// `4x8x8`. Standard grids carry no tag and use `None`.
    #[serde(default)]
    pub tag: Option<String>,
    pub size: usize,
    pub regions: Vec<Region>,
}

impl Geometry {
    /// A plain n×n puzzle: one region, the whole grid.
    pub fn standard(n: usize) -> Result<Self, GeometryError> {
        let geometry = Geometry {
            tag: None,
            size: n,
            regions: vec![Region::square(0, 0, n)],
        };
        geometry.validate()?;
        Ok(geometry)
    }

    /// Parse a geometry from TOML text. Reading the file is the CLI's job
    /// (AR1): the core only ever sees the text.
    pub fn from_toml(text: &str) -> Result<Self, GeometryError> {
        let geometry: Geometry = toml::from_str(text).map_err(|e| GeometryError::Syntax {
            detail: e.message().to_string(),
        })?;
        geometry.validate()?;
        Ok(geometry)
    }

    /// Structural validation (K4). Feasibility is a different question
    /// and needs the solver, so it is not answered here — see
    /// `GeometryError::ProvedInfeasible`, produced from L3.
    pub fn validate(&self) -> Result<(), GeometryError> {
        if self.size == 0 {
            return Err(GeometryError::GridSize {
                size: self.size,
                reason: SizeProblem::Zero,
            });
        }
        if !self.size.is_multiple_of(2) {
            return Err(GeometryError::GridSize {
                size: self.size,
                reason: SizeProblem::Odd,
            });
        }
        if self.size > MAX_SUPPORTED_SIZE {
            return Err(GeometryError::GridSize {
                size: self.size,
                reason: SizeProblem::TooLarge {
                    limit: MAX_SUPPORTED_SIZE,
                },
            });
        }
        if self.regions.is_empty() {
            return Err(GeometryError::NoRegions);
        }
        for (index, region) in self.regions.iter().enumerate() {
            if region.rows == 0 || region.cols == 0 {
                return Err(GeometryError::RegionEmpty { region: index });
            }
            if region.row + region.rows > self.size || region.col + region.cols > self.size {
                return Err(GeometryError::RegionOutOfBounds {
                    region: index,
                    row: region.row,
                    col: region.col,
                    rows: region.rows,
                    cols: region.cols,
                    size: self.size,
                });
            }
            // A balanced line needs equal counts of 0 and 1, which an odd
            // length cannot hold. Only enforced where the rule applies, so
            // a region that opts out may be any shape.
            if region.rules.balance
                && (!region.rows.is_multiple_of(2) || !region.cols.is_multiple_of(2))
            {
                return Err(GeometryError::RegionSideOdd {
                    region: index,
                    rows: region.rows,
                    cols: region.cols,
                });
            }
        }
        Ok(())
    }

    /// The smallest region covering a cell, by area, with earlier regions
    /// winning ties. Drives the inspect rendering (M6): the innermost
    /// region is the one a reader needs to see.
    pub fn innermost_at(&self, row: usize, col: usize) -> Option<usize> {
        self.regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.contains(row, col))
            .min_by_key(|(index, r)| (r.area(), *index))
            .map(|(index, _)| index)
    }

    /// How many regions cover a cell. A cell covered by none is a hole:
    /// no rule constrains it, which is almost always a mistake.
    pub fn coverage_at(&self, row: usize, col: usize) -> usize {
        self.regions.iter().filter(|r| r.contains(row, col)).count()
    }

    /// Cells that no region constrains. Structurally legal — the grid
    /// still has a whole-grid region in every real type — but worth
    /// surfacing, since it is what a mistyped origin produces.
    pub fn uncovered_cells(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for row in 0..self.size {
            for col in 0..self.size {
                if self.coverage_at(row, col) == 0 {
                    out.push((row, col));
                }
            }
        }
        out
    }
}

/// The five composite types from binarypuzzle.com, embedded at build
/// time (AR3) so a default install needs no external files. Invented
/// types are loaded from a file instead — same model, same validation.
pub const BUILTIN_TAGS: [&str; 5] = ["4x6x6", "4x8x8", "9x6x6", "8in14", "6in10in14"];

const BUILTIN_SOURCES: [(&str, &str); 5] = [
    ("4x6x6", include_str!("../../geometries/4x6x6.toml")),
    ("4x8x8", include_str!("../../geometries/4x8x8.toml")),
    ("9x6x6", include_str!("../../geometries/9x6x6.toml")),
    ("8in14", include_str!("../../geometries/8in14.toml")),
    ("6in10in14", include_str!("../../geometries/6in10in14.toml")),
];

/// Look up a built-in type by its tag. Unknown tags return `None`; the
/// caller decides whether that is an error or a cue to read a file.
pub fn builtin(tag: &str) -> Option<Result<Geometry, GeometryError>> {
    BUILTIN_SOURCES
        .iter()
        .find(|(name, _)| *name == tag)
        .map(|(_, source)| Geometry::from_toml(source))
}
