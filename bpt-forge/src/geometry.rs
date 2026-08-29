//! K23: a puzzle type is data — a grid size plus the rectangular regions
//! that must each satisfy the Takuzu rules. Built-in types and invented
//! ones are entries in the same model, so a new type is a TOML file
//! rather than a code change here.
//!
//! Regions are rectangular (AR22b) although every type known today is
//! square: the alternative was migrating hand-written geometry files
//! later, and overlapping-band compositions are the one genuinely novel
//! family in scope (G3).

// The generator constructs regions; the solver consumes them. One
// definition, in the shared core, so the two can never drift apart.
use bpt_core::region::{Region, RuleSet};

/// How a region is written in a geometry file. Kept separate from
/// `bpt_core::region::Region` on purpose: the core carries no
/// dependencies at all, so it cannot derive serde, and the file format
/// is a generator concern rather than part of the domain model. This is
/// the only place the two representations meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionSpec {
    pub row: usize,
    pub col: usize,
    pub rows: usize,
    pub cols: usize,
    #[serde(default)]
    pub rules: RuleSetSpec,
}

/// The file form of a rule set; every rule is on unless a file says
/// otherwise, so omitting the field means "all rules apply".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSetSpec {
    #[serde(default = "yes")]
    pub balance: bool,
    #[serde(default = "yes")]
    pub no_triples: bool,
    #[serde(default = "yes")]
    pub unique_lines: bool,
}

fn yes() -> bool {
    true
}

impl RuleSetSpec {
    /// Every rule on — the default, and what all six known kinds use.
    pub const ALL: RuleSetSpec = RuleSetSpec {
        balance: true,
        no_triples: true,
        unique_lines: true,
    };
}

impl Default for RuleSetSpec {
    fn default() -> Self {
        RuleSetSpec {
            balance: true,
            no_triples: true,
            unique_lines: true,
        }
    }
}

impl From<RuleSetSpec> for RuleSet {
    fn from(s: RuleSetSpec) -> Self {
        RuleSet {
            balance: s.balance,
            no_triples: s.no_triples,
            unique_lines: s.unique_lines,
        }
    }
}

impl RegionSpec {
    pub fn square(row: usize, col: usize, n: usize) -> Self {
        RegionSpec {
            row,
            col,
            rows: n,
            cols: n,
            rules: RuleSetSpec::default(),
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

impl From<RegionSpec> for Region {
    fn from(s: RegionSpec) -> Self {
        Region {
            row: s.row,
            col: s.col,
            rows: s.rows,
            cols: s.cols,
            rules: s.rules.into(),
        }
    }
}
use crate::error::{GeometryError, SizeProblem};
use serde::Deserialize;

/// A resource guard against absurd input, NOT the practical size ceiling:
/// scope G1 puts that ceiling in the G8 performance measurements and says
/// it is never hardcoded. Without any bound a file saying
/// `size = 9223372036854775806` would send the renderer and the coverage
/// scan into a loop over 2^63 rows, so the guard sits far above any
/// plausible puzzle (256x256 = 65 536 cells) and refuses only nonsense.
pub const MAX_SUPPORTED_SIZE: usize = 256;

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
    pub regions: Vec<RegionSpec>,
}

/// How many distinct balanced lines of length `n` exist: C(n, n/2).
///
/// Saturating, because the answer only matters while it is small — once
/// it exceeds any plausible region width the comparison is settled, and
/// a 20-long line already allows 184 756.
fn balanced_lines(n: usize) -> usize {
    let half = n / 2;
    let mut result: usize = 1;
    for i in 0..half {
        result = result.saturating_mul(n - i) / (i + 1);
        if result > MAX_SUPPORTED_SIZE {
            return usize::MAX;
        }
    }
    result
}

impl Geometry {
    /// The regions as the solver and the generator both understand them.
    pub fn to_regions(&self) -> Vec<Region> {
        self.regions.iter().copied().map(Region::from).collect()
    }

    /// A plain n×n puzzle: one region, the whole grid.
    pub fn standard(n: usize) -> Result<Self, GeometryError> {
        let geometry = Geometry {
            tag: None,
            size: n,
            regions: vec![RegionSpec::square(0, 0, n)],
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

    /// Structural validation (K23). Feasibility is a different question
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
            // A flat region can run out of lines to be different from.
            // There are only C(h, h/2) balanced lines of length h, so a
            // region with more than that many of them cannot keep them
            // all distinct — no matter how it is filled.
            //
            // Found by trying it: three 4x12 bands in a 12x12 need twelve
            // distinct balanced columns of height 4, and only six exist.
            // Before this check that was a search that ran and then
            // reported "no solution, no seed will help"; now it is a
            // structural answer with a reason (AR32.5).
            if region.rules.balance && region.rules.unique_lines {
                for along_columns in [true, false] {
                    let (length, count) = if along_columns {
                        (region.rows, region.cols)
                    } else {
                        (region.cols, region.rows)
                    };
                    let available = balanced_lines(length);
                    if count > available {
                        return Err(GeometryError::RegionTooFlat {
                            region: index,
                            rows: region.rows,
                            cols: region.cols,
                            needed: count,
                            available,
                            along_columns,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// The smallest region covering a cell, by area, with earlier regions
    /// winning ties. Drives the inspect rendering (M25): the innermost
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
