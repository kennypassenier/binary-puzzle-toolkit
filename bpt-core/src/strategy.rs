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
    KeepLineUnique,
    CountingArgument,
    FillPossibilities,
}

impl StrategyId {
    pub fn name(&self) -> &'static str {
        match self {
            StrategyId::FindDuo => "FindDuo",
            StrategyId::AvoidTriple => "AvoidTriple",
            StrategyId::FillByCount => "FillByCount",
            StrategyId::KeepLineUnique => "KeepLineUnique",
            StrategyId::CountingArgument => "CountingArgument",
            StrategyId::FillPossibilities => "FillPossibilities",
        }
    }

    /// Complexity tier (AR5): 1–2 are constant-cost and may propagate
    /// inside DFS; 3–4 run only between search episodes.
    pub fn tier(&self) -> u8 {
        match self {
            StrategyId::FindDuo | StrategyId::AvoidTriple => 1,
            StrategyId::FillByCount => 2,
            StrategyId::KeepLineUnique | StrategyId::CountingArgument => 3,
            StrategyId::FillPossibilities => 4,
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
    /// Filling the two open cells like the completed line `other` would
    /// duplicate it; both must take the opposite values.
    UniqueSwap { is_row: bool, other: usize },
    /// Placing `tried` here leaves no valid completion of the line
    /// under the count and no-triple rules.
    CountInfeasible { tried: Cell },
    /// Every rule-valid, non-duplicating completion of the line agrees
    /// on this cell.
    Enumeration { survivors: usize },
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
    grid: &'a Grid,
}

impl<'a> LineView<'a> {
    pub fn new(grid: &'a Grid, region: Region, is_row: bool, index: usize) -> Self {
        LineView {
            cells: region.line_cells(grid, is_row, index),
            region,
            is_row,
            index,
            grid,
        }
    }

    /// Completed parallel lines of the same region (uniqueness rule
    /// context), in fixed region order (AR13), with their absolute
    /// line index.
    pub fn completed_parallel(&self) -> Vec<(usize, Vec<Cell>)> {
        (0..self.region.n)
            .filter(|i| *i != self.index)
            .filter_map(|i| {
                let cells = self.region.line_cells(self.grid, self.is_row, i);
                if cells.iter().all(|c| !c.is_empty()) {
                    let abs = if self.is_row {
                        self.region.row + i
                    } else {
                        self.region.col + i
                    };
                    Some((abs, cells))
                } else {
                    None
                }
            })
            .collect()
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

pub struct KeepLineUnique;

impl Strategy for KeepLineUnique {
    fn id(&self) -> StrategyId {
        StrategyId::KeepLineUnique
    }

    fn apply(&self, line: &LineView<'_>) -> Vec<Deduction> {
        let cells = line.cells();
        let empties: Vec<usize> = (0..cells.len()).filter(|i| cells[*i].is_empty()).collect();
        if empties.len() != 2 {
            return Vec::new();
        }
        for (other, complete) in line.completed_parallel() {
            let agrees = (0..cells.len()).all(|i| cells[i].is_empty() || cells[i] == complete[i]);
            if !agrees {
                continue;
            }
            let (a, b) = (complete[empties[0]], complete[empties[1]]);
            if a == b {
                // Copying is unavoidable on counts alone; that is a
                // contradiction for search to surface, not a deduction.
                continue;
            }
            let reason = Reason::UniqueSwap {
                is_row: line.is_row(),
                other,
            };
            return empties
                .iter()
                .map(|&i| {
                    let (row, col) = line.pos(i);
                    Deduction {
                        row,
                        col,
                        value: complete[i]
                            .opposite()
                            .expect("complete line has no empties"),
                        strategy: self.id(),
                        reason,
                    }
                })
                .collect();
        }
        Vec::new()
    }
}

/// Is there any completion of `cells` with exactly n/2 of each value
/// and no three-in-a-row? Small DP over (position, zeros used, run).
fn can_complete(cells: &[Cell]) -> bool {
    fn go(
        cells: &[Cell],
        pos: usize,
        zeros: usize,
        last: Cell,
        run: usize,
        memo: &mut [Option<bool>],
        need: usize,
    ) -> bool {
        if zeros > need || (pos - zeros) > need {
            return false;
        }
        if pos == cells.len() {
            return zeros == need;
        }
        let last_ix = match last {
            Cell::Zero => 0,
            Cell::One => 1,
            Cell::Empty => 2,
        };
        let key = ((pos * (need + 1) + zeros) * 3 + last_ix) * 2 + (run.min(2) - 1);
        if let Some(hit) = memo[key] {
            return hit;
        }
        let mut ok = false;
        for value in [Cell::Zero, Cell::One] {
            if !cells[pos].is_empty() && cells[pos] != value {
                continue;
            }
            if value == last && run >= 2 {
                continue;
            }
            let new_run = if value == last { run + 1 } else { 1 };
            let new_zeros = zeros + usize::from(value == Cell::Zero);
            if go(cells, pos + 1, new_zeros, value, new_run, memo, need) {
                ok = true;
                break;
            }
        }
        memo[key] = Some(ok);
        ok
    }
    let need = cells.len() / 2;
    let mut memo = vec![None; (cells.len() + 1) * (need + 1) * 3 * 2];
    go(cells, 0, 0, Cell::Empty, 1, &mut memo, need)
}

pub struct CountingArgument;

impl Strategy for CountingArgument {
    fn id(&self) -> StrategyId {
        StrategyId::CountingArgument
    }

    fn apply(&self, line: &LineView<'_>) -> Vec<Deduction> {
        let cells = line.cells();
        let mut out = Vec::new();
        let mut scratch = cells.to_vec();
        for i in 0..cells.len() {
            if !cells[i].is_empty() {
                continue;
            }
            for tried in [Cell::Zero, Cell::One] {
                scratch[i] = tried;
                let feasible = can_complete(&scratch);
                scratch[i] = Cell::Empty;
                if !feasible {
                    let (row, col) = line.pos(i);
                    out.push(Deduction {
                        row,
                        col,
                        value: tried.opposite().expect("tried is filled"),
                        strategy: self.id(),
                        reason: Reason::CountInfeasible { tried },
                    });
                }
            }
        }
        out
    }
}

pub struct FillPossibilities;

impl Strategy for FillPossibilities {
    fn id(&self) -> StrategyId {
        StrategyId::FillPossibilities
    }

    fn apply(&self, line: &LineView<'_>) -> Vec<Deduction> {
        let cells = line.cells();
        if cells.iter().all(|c| !c.is_empty()) {
            return Vec::new();
        }
        let completed = line.completed_parallel();
        let mut survivors = 0usize;
        let mut forced: Vec<Option<Cell>> = Vec::new();
        let mut candidate = cells.to_vec();
        enumerate(&mut candidate, 0, cells, &mut |full| {
            // The uniqueness filter (N3): completions that would copy a
            // finished parallel line are not legal solutions.
            if completed.iter().any(|(_, other)| other == full) {
                return;
            }
            survivors += 1;
            if forced.is_empty() {
                forced = full.iter().map(|c| Some(*c)).collect();
            } else {
                for (slot, value) in forced.iter_mut().zip(full) {
                    if *slot != Some(*value) {
                        *slot = None;
                    }
                }
            }
        });
        if survivors == 0 {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (i, slot) in forced.iter().enumerate() {
            if let Some(value) = slot
                && cells[i].is_empty()
            {
                let (row, col) = line.pos(i);
                out.push(Deduction {
                    row,
                    col,
                    value: *value,
                    strategy: self.id(),
                    reason: Reason::Enumeration { survivors },
                });
            }
        }
        out
    }
}

/// Enumerate all completions of `candidate` (mutated in place) that
/// satisfy counts and no-triples, invoking `sink` for each.
fn enumerate(
    candidate: &mut Vec<Cell>,
    pos: usize,
    givens: &[Cell],
    sink: &mut impl FnMut(&[Cell]),
) {
    let n = givens.len();
    let need = n / 2;
    if pos == n {
        sink(candidate);
        return;
    }
    let zeros = candidate[..pos]
        .iter()
        .filter(|c| **c == Cell::Zero)
        .count();
    for value in [Cell::Zero, Cell::One] {
        if !givens[pos].is_empty() && givens[pos] != value {
            continue;
        }
        if pos >= 2 && candidate[pos - 1] == value && candidate[pos - 2] == value {
            continue;
        }
        let new_zeros = zeros + usize::from(value == Cell::Zero);
        let new_ones = pos + 1 - new_zeros;
        if new_zeros > need || new_ones > need {
            continue;
        }
        candidate[pos] = value;
        enumerate(candidate, pos + 1, givens, sink);
    }
    candidate[pos] = givens[pos];
}

/// All strategies in fixed tier order (AR13 determinism).
pub fn registry() -> Vec<Box<dyn Strategy>> {
    vec![
        Box::new(FindDuo),
        Box::new(AvoidTriple),
        Box::new(FillByCount),
        Box::new(KeepLineUnique),
        Box::new(CountingArgument),
        Box::new(FillPossibilities),
    ]
}

/// Strategies grouped by escalation stage: cheap (tiers 1–2, also the
/// in-DFS set per AR5), cross-line (tier 3), enumeration (tier 4). The
/// engine exhausts a stage before escalating (Tatham's ladder).
pub fn registry_stages() -> Vec<Vec<Box<dyn Strategy>>> {
    vec![
        vec![
            Box::new(FindDuo) as Box<dyn Strategy>,
            Box::new(AvoidTriple),
            Box::new(FillByCount),
        ],
        vec![
            Box::new(KeepLineUnique) as Box<dyn Strategy>,
            Box::new(CountingArgument),
        ],
        vec![Box::new(FillPossibilities) as Box<dyn Strategy>],
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
    fn k3_keep_line_unique_readme_scenario() {
        // README: 10.. -> 1010 when another line has 1001.
        let mut cells = vec![Cell::Empty; 16];
        for (i, c) in parse_grid_chars("1001").unwrap().into_iter().enumerate() {
            cells[i] = c;
        }
        for (i, c) in parse_grid_chars("10..").unwrap().into_iter().enumerate() {
            cells[4 + i] = c;
        }
        let mut grid = Grid::from_cells(4, cells);
        let region = Region {
            row: 0,
            col: 0,
            n: 4,
            rules: RuleSet::ALL,
        };
        let view = LineView::new(&grid, region, true, 1);
        let deductions = KeepLineUnique.apply(&view);
        assert_eq!(deductions.len(), 2);
        for d in &deductions {
            grid.set(d.row, d.col, d.value);
        }
        let row1: String = (0..4).map(|c| grid.get(1, c).to_char()).collect();
        assert_eq!(row1, "1010");
    }

    #[test]
    fn k3_counting_argument_virtual_limit_readme_scenario() {
        // README (VirtualLimitReached): 001..0.. — the digits after the
        // last 0 must all be 1.
        assert_eq!(apply_to_line(&CountingArgument, "001..0.."), "001..011");
    }

    #[test]
    fn k3_fill_possibilities_readme_scenarios() {
        // The old README claimed 01...010 forces its first unknown to 0.
        // Enumeration disproves it: 01011010 and 01101010 are both
        // valid, so position 2 is ambiguous — only position 4 is forced
        // (both survivors have 1 there).
        let out = apply_to_line(&FillPossibilities, "01...010");
        assert_eq!(out, "01..1010");
        // README: 0....0 -> 1 on both inner edges.
        let out = apply_to_line(&FillPossibilities, "0....0");
        assert_eq!(out.chars().nth(1).unwrap(), '1', "{out}");
        assert_eq!(out.chars().nth(4).unwrap(), '1', "{out}");
        // README: 001... -> 001..1.
        let out = apply_to_line(&FillPossibilities, "001...");
        assert_eq!(out.chars().nth(5).unwrap(), '1', "{out}");
    }

    #[test]
    fn k3_fill_possibilities_uniqueness_filter_site_example() {
        // Site "solving difficult puzzles" pattern: counts+triples allow
        // both fillings of row 1, but one would duplicate finished row 0
        // — the filter forces the swap.
        let mut cells = vec![Cell::Empty; 36];
        for (i, c) in parse_grid_chars("010011").unwrap().into_iter().enumerate() {
            cells[i] = c;
        }
        for (i, c) in parse_grid_chars("010..1").unwrap().into_iter().enumerate() {
            cells[6 + i] = c;
        }
        let mut grid = Grid::from_cells(6, cells);
        let region = Region {
            row: 0,
            col: 0,
            n: 6,
            rules: RuleSet::ALL,
        };
        let view = LineView::new(&grid, region, true, 1);
        let deductions = FillPossibilities.apply(&view);
        for d in &deductions {
            grid.set(d.row, d.col, d.value);
        }
        let row1: String = (0..6).map(|c| grid.get(1, c).to_char()).collect();
        assert_eq!(row1, "010101");
    }

    #[test]
    fn k3_can_complete_dp() {
        let parse = |s: &str| parse_grid_chars(s).unwrap();
        assert!(can_complete(&parse("......")));
        assert!(can_complete(&parse("0101..")));
        // Four zeros in a 6-line: over the count limit.
        assert!(!can_complete(&parse("00100.")));
        // Given triple.
        assert!(!can_complete(&parse("000...")));
        // Counts force both empties to 0, creating a triple: the DP has
        // to see the interaction, not just each rule alone.
        assert!(!can_complete(&parse("1.1100.1")));
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
