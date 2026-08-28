//! AR6/AR13.5: every error carries a remedy, and "proved infeasible" is
//! a different answer from "gave up after N work units" — K4's tests
//! depend on telling those apart.

use std::fmt;

/// Why a geometry could not be used. Structural problems (K4) are
/// detected without a solver; the feasibility variants exist here so the
/// distinction is part of the type from the start, and are produced from
/// L3 onwards when the filler can actually attempt a grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeometryError {
    /// The TOML did not parse at all.
    Syntax { detail: String },
    /// The grid side length is unusable: odd, zero, or absurdly large.
    GridSize { size: usize, reason: SizeProblem },
    /// A region falls outside the grid it belongs to.
    RegionOutOfBounds {
        region: usize,
        row: usize,
        col: usize,
        rows: usize,
        cols: usize,
        size: usize,
    },
    /// A region has a side length that cannot hold equal counts of 0 and
    /// 1, so its balance rule can never be satisfied.
    RegionSideOdd {
        region: usize,
        rows: usize,
        cols: usize,
    },
    /// A region has no extent at all.
    RegionEmpty { region: usize },
    /// No regions were declared, so nothing constrains the grid.
    NoRegions,
    /// The solver could not complete an empty grid: the rules of these
    /// regions contradict each other. Produced from L3.
    ProvedInfeasible { detail: String },
    /// The solver ran out of its work-unit budget without deciding.
    /// Explicitly NOT the same answer as `ProvedInfeasible` (AR13.5).
    BudgetExhausted { work_units: u64 },
}

/// What is wrong with a grid side length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeProblem {
    Zero,
    Odd,
    TooLarge { limit: usize },
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GeometryError::Syntax { detail } => write!(
                f,
                "geometry file is not valid TOML: {detail}\n\
                 Remedy: check for a missing quote or bracket; every region \
                 is a [[regions]] table with row, col, rows and cols."
            ),
            GeometryError::GridSize { size, reason } => match reason {
                SizeProblem::Zero => write!(
                    f,
                    "grid size is 0\n\
                     Remedy: set size to an even number of at least 2, e.g. size = 6."
                ),
                SizeProblem::Odd => write!(
                    f,
                    "grid size {size} is odd\n\
                     Remedy: binary puzzles need equal counts of 0 and 1 per line, \
                     so the side length must be even — use {} or {}.",
                    size - 1,
                    size + 1
                ),
                SizeProblem::TooLarge { limit } => write!(
                    f,
                    "grid size {size} exceeds the supported limit of {limit}\n\
                     Remedy: use a smaller size; generation time grows steeply \
                     with area and nothing above {limit} has been measured."
                ),
            },
            GeometryError::RegionOutOfBounds {
                region,
                row,
                col,
                rows,
                cols,
                size,
            } => write!(
                f,
                "region {region} covers rows {row}..{} and columns {col}..{}, \
                 which falls outside the {size}x{size} grid\n\
                 Remedy: keep row + rows and col + cols within {size}; \
                 origins are 0-based.",
                row + rows,
                col + cols
            ),
            GeometryError::RegionSideOdd { region, rows, cols } => write!(
                f,
                "region {region} is {rows}x{cols}: a side length is odd\n\
                 Remedy: every region needs equal counts of 0 and 1 in each of \
                 its lines, which is impossible on an odd length — make both \
                 sides even, or drop the balance rule for this region."
            ),
            GeometryError::RegionEmpty { region } => write!(
                f,
                "region {region} has zero width or height\n\
                 Remedy: give it a positive rows and cols, or remove it."
            ),
            GeometryError::NoRegions => write!(
                f,
                "the geometry declares no regions, so no rule would apply anywhere\n\
                 Remedy: add at least the whole-grid region, e.g. a [[regions]] \
                 table with row = 0, col = 0 and rows = cols = size."
            ),
            GeometryError::ProvedInfeasible { detail } => write!(
                f,
                "this geometry has no solution at all: {detail}\n\
                 Remedy: the regions contradict each other — relax an overlap or \
                 drop a rule; this is a property of the shape, not of one attempt."
            ),
            GeometryError::BudgetExhausted { work_units } => write!(
                f,
                "gave up after {work_units} work units without deciding whether \
                 this geometry is solvable\n\
                 Remedy: this is NOT proof that it is impossible — raise the \
                 budget to let it run longer, or simplify the geometry."
            ),
        }
    }
}

impl std::error::Error for GeometryError {}
