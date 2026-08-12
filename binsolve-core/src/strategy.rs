//! The AR4 strategy engine: pure, attributable deductions on lines of
//! regions. Tier numbers feed M2 difficulty grading; reasons are
//! structured and rendered lazily (never eager strings in hot loops).

use crate::grid::{Cell, Grid};
use crate::region::Region;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyId {
    FindDuo,
    AvoidTriple,
    FillByCount,
}

impl StrategyId {
    pub fn name(&self) -> &'static str {
        match self {
            StrategyId::FindDuo => "FindDuo",
            StrategyId::AvoidTriple => "AvoidTriple",
            StrategyId::FillByCount => "FillByCount",
        }
    }

    /// Complexity tier (AR5): 1–2 are constant-cost and may propagate
    /// inside DFS; 3–4 run only between search episodes.
    pub fn tier(&self) -> u8 {
        match self {
            StrategyId::FindDuo | StrategyId::AvoidTriple => 1,
            StrategyId::FillByCount => 2,
        }
    }
}

/// Why a deduction holds, in grid coordinates (AR4: structured, lazy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Two adjacent equal cells force their neighbours to differ.
    AdjacentPair {
        a: (usize, usize),
        b: (usize, usize),
        value: Cell,
    },
    /// Equal cells with one gap force the middle to differ.
    Gap {
        a: (usize, usize),
        b: (usize, usize),
        value: Cell,
    },
    /// A line that already holds all its copies of `value` forces the
    /// rest to the opposite.
    CountComplete {
        is_row: bool,
        line: usize,
        value: Cell,
        need: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Deduction {
    pub row: usize,
    pub col: usize,
    pub value: Cell,
    pub strategy: StrategyId,
    pub reason: Reason,
}

/// One line of one region, with the mapping back to grid coordinates.
pub struct LineView<'a> {
    cells: Vec<Cell>,
    region: Region,
    is_row: bool,
    index: usize,
    _grid: &'a Grid,
}

impl<'a> LineView<'a> {
    pub fn new(grid: &'a Grid, region: Region, is_row: bool, index: usize) -> Self {
        LineView {
            cells: region.line_cells(grid, is_row, index),
            region,
            is_row,
            index,
            _grid: grid,
        }
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Grid coordinates of position `i` within this line.
    pub fn pos(&self, i: usize) -> (usize, usize) {
        if self.is_row {
            (self.region.row + self.index, self.region.col + i)
        } else {
            (self.region.row + i, self.region.col + self.index)
        }
    }

    /// Absolute index of this line in the grid (row or column number).
    pub fn line_index(&self) -> usize {
        if self.is_row {
            self.region.row + self.index
        } else {
            self.region.col + self.index
        }
    }

    pub fn is_row(&self) -> bool {
        self.is_row
    }
}

pub trait Strategy {
    fn id(&self) -> StrategyId;
    fn apply(&self, line: &LineView<'_>) -> Vec<Deduction>;
}

pub struct FindDuo;

impl Strategy for FindDuo {
    fn id(&self) -> StrategyId {
        StrategyId::FindDuo
    }

    fn apply(&self, line: &LineView<'_>) -> Vec<Deduction> {
        let cells = line.cells();
        let mut out = Vec::new();
        for i in 0..cells.len().saturating_sub(1) {
            let value = cells[i];
            if value.is_empty() || cells[i + 1] != value {
                continue;
            }
            let forced = value.opposite().expect("filled cell has an opposite");
            let reason = Reason::AdjacentPair {
                a: line.pos(i),
                b: line.pos(i + 1),
                value,
            };
            if i > 0 && cells[i - 1].is_empty() {
                let (row, col) = line.pos(i - 1);
                out.push(Deduction {
                    row,
                    col,
                    value: forced,
                    strategy: self.id(),
                    reason,
                });
            }
            if i + 2 < cells.len() && cells[i + 2].is_empty() {
                let (row, col) = line.pos(i + 2);
                out.push(Deduction {
                    row,
                    col,
                    value: forced,
                    strategy: self.id(),
                    reason,
                });
            }
        }
        out
    }
}

pub struct AvoidTriple;

impl Strategy for AvoidTriple {
    fn id(&self) -> StrategyId {
        StrategyId::AvoidTriple
    }

    fn apply(&self, line: &LineView<'_>) -> Vec<Deduction> {
        let cells = line.cells();
        let mut out = Vec::new();
        for i in 0..cells.len().saturating_sub(2) {
            let value = cells[i];
            if value.is_empty() || cells[i + 2] != value || !cells[i + 1].is_empty() {
                continue;
            }
            let (row, col) = line.pos(i + 1);
            out.push(Deduction {
                row,
                col,
                value: value.opposite().expect("filled cell has an opposite"),
                strategy: self.id(),
                reason: Reason::Gap {
                    a: line.pos(i),
                    b: line.pos(i + 2),
                    value,
                },
            });
        }
        out
    }
}

pub struct FillByCount;

impl Strategy for FillByCount {
    fn id(&self) -> StrategyId {
        StrategyId::FillByCount
    }

    fn apply(&self, line: &LineView<'_>) -> Vec<Deduction> {
        let cells = line.cells();
        let need = cells.len() / 2;
        let mut out = Vec::new();
        for value in [Cell::Zero, Cell::One] {
            let count = cells.iter().filter(|c| **c == value).count();
            if count != need || cells.iter().all(|c| !c.is_empty()) {
                continue;
            }
            let forced = value.opposite().expect("filled cell has an opposite");
            for (i, cell) in cells.iter().enumerate() {
                if cell.is_empty() {
                    let (row, col) = line.pos(i);
                    out.push(Deduction {
                        row,
                        col,
                        value: forced,
                        strategy: self.id(),
                        reason: Reason::CountComplete {
                            is_row: line.is_row(),
                            line: line.line_index(),
                            value,
                            need,
                        },
                    });
                }
            }
        }
        out
    }
}

/// All strategies in fixed tier order (AR13 determinism).
pub fn registry() -> Vec<Box<dyn Strategy>> {
    vec![
        Box::new(FindDuo),
        Box::new(AvoidTriple),
        Box::new(FillByCount),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_grid_chars;
    use crate::region::{PuzzleKind, RuleSet};

    /// Apply one strategy to a single line laid out as row 0 of a grid,
    /// returning the line after applying the deductions.
    fn apply_to_line(strategy: &dyn Strategy, line: &str) -> String {
        let n = line.len();
        let mut cells = vec![Cell::Empty; n * n];
        for (i, c) in parse_grid_chars(line).unwrap().into_iter().enumerate() {
            cells[i] = c;
        }
        let mut grid = Grid::from_cells(n, cells);
        let region = Region {
            row: 0,
            col: 0,
            n,
            rules: RuleSet::ALL,
        };
        let view = LineView::new(&grid, region, true, 0);
        let deductions = strategy.apply(&view);
        for d in deductions {
            grid.set(d.row, d.col, d.value);
        }
        (0..n).map(|c| grid.get(0, c).to_char()).collect()
    }

    #[test]
    fn k3_find_duo_readme_scenario() {
        // README: .00. -> 100.  (one application fills both neighbours)
        assert_eq!(apply_to_line(&FindDuo, ".00."), "1001");
        assert_eq!(apply_to_line(&FindDuo, ".11."), "0110");
        assert_eq!(apply_to_line(&FindDuo, "00.."), "001.");
    }

    #[test]
    fn k3_avoid_triple_readme_scenario() {
        assert_eq!(apply_to_line(&AvoidTriple, "0.0"), "010");
        assert_eq!(apply_to_line(&AvoidTriple, "1.1"), "101");
        assert_eq!(apply_to_line(&AvoidTriple, "0.1"), "0.1");
    }

    #[test]
    fn k3_fill_by_count() {
        // Three zeros present in a 6-line: the rest must be ones.
        assert_eq!(apply_to_line(&FillByCount, "00.0.."), "001011");
        assert_eq!(apply_to_line(&FillByCount, "11.1.."), "110100");
        assert_eq!(apply_to_line(&FillByCount, "01...."), "01....");
    }

    #[test]
    fn k2_line_view_maps_region_coordinates() {
        let grid = Grid::empty(14);
        let inner = PuzzleKind::SixIn10In14.regions()[0];
        let view = LineView::new(&grid, inner, true, 2);
        // Row 2 of the centered 6x6 is grid row 6, columns 4..10.
        assert_eq!(view.pos(0), (6, 4));
        assert_eq!(view.pos(5), (6, 9));
        assert_eq!(view.line_index(), 6);
    }
}
