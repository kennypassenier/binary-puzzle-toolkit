//! Cell and Grid: the AR2 representation — a flat row-major `Vec<Cell>`.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Zero,
    One,
    Empty,
}

impl Cell {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '0' => Some(Cell::Zero),
            '1' => Some(Cell::One),
            '.' => Some(Cell::Empty),
            _ => None,
        }
    }

    pub fn to_char(self) -> char {
        match self {
            Cell::Zero => '0',
            Cell::One => '1',
            Cell::Empty => '.',
        }
    }

    pub fn is_empty(self) -> bool {
        self == Cell::Empty
    }

    /// The other filled value; Empty has no opposite.
    pub fn opposite(self) -> Option<Self> {
        match self {
            Cell::Zero => Some(Cell::One),
            Cell::One => Some(Cell::Zero),
            Cell::Empty => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    n: usize,
    cells: Vec<Cell>,
}

impl Grid {
    pub fn empty(n: usize) -> Self {
        Grid {
            n,
            cells: vec![Cell::Empty; n * n],
        }
    }

    /// `cells` must contain exactly n*n entries.
    pub fn from_cells(n: usize, cells: Vec<Cell>) -> Self {
        assert_eq!(cells.len(), n * n, "grid needs n*n cells");
        Grid { n, cells }
    }

    pub fn size(&self) -> usize {
        self.n
    }

    pub fn get(&self, r: usize, c: usize) -> Cell {
        self.cells[r * self.n + c]
    }

    pub fn set(&mut self, r: usize, c: usize, value: Cell) {
        self.cells[r * self.n + c] = value;
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn filled_count(&self) -> usize {
        self.cells.iter().filter(|c| !c.is_empty()).count()
    }

    pub fn is_complete(&self) -> bool {
        self.cells.iter().all(|c| !c.is_empty())
    }

    /// Canonical single-line serialization (AR7 grid part).
    pub fn to_line(&self) -> String {
        self.cells.iter().map(|c| c.to_char()).collect()
    }
}

/// Human-readable multi-line rendering (K11 pretty output builds on this).
impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for r in 0..self.n {
            for c in 0..self.n {
                if c > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{}", self.get(r, c).to_char())?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k7_cell_char_round_trip() {
        for c in ['0', '1', '.'] {
            assert_eq!(Cell::from_char(c).unwrap().to_char(), c);
        }
        assert_eq!(Cell::from_char('x'), None);
        assert_eq!(Cell::from_char('\r'), None);
    }

    #[test]
    fn k7_grid_indexing_row_major() {
        let mut g = Grid::empty(6);
        g.set(2, 3, Cell::One);
        assert_eq!(g.get(2, 3), Cell::One);
        assert_eq!(g.cells()[2 * 6 + 3], Cell::One);
        assert_eq!(g.filled_count(), 1);
        assert!(!g.is_complete());
    }

    #[test]
    fn k7_grid_line_round_trip() {
        let line = "10..01..1";
        let cells: Vec<Cell> = line.chars().map(|c| Cell::from_char(c).unwrap()).collect();
        let g = Grid::from_cells(3, cells);
        assert_eq!(g.to_line(), line);
    }
}
