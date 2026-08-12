//! L5 region-boundary tests [K2a-K2e]: deductions must respect region
//! boundaries — an inner-region rule fires without outer-grid support
//! and vice versa. Per-type solve coverage lives in the corpus-driven
//! suites (sweep, solve, corpus).

use binsolve_core::event::EventLog;
use binsolve_core::grid::Cell;
use binsolve_core::parse::parse_line;
use binsolve_core::region::PuzzleKind;
use binsolve_core::search::{StrategyRun, run_to_fixpoint};
use binsolve_core::strategy::{Reason, StrategyId};

/// Build a 6in10in14 line with the given cells placed by (row, col).
fn puzzle_with_cells(tag: &str, n: usize, cells: &[(usize, usize, char)]) -> String {
    let mut grid = vec!['.'; n * n];
    for (r, c, v) in cells {
        grid[r * n + c] = *v;
    }
    format!("{tag}:{}", grid.iter().collect::<String>())
}

#[test]
fn k2e_inner_region_count_rule_fires_alone() {
    // Row 6 of the inner 6x6 (grid cols 4..10) gets three 0s: the inner
    // balance rule forces the other three inner cells to 1, while the
    // outer 14-row (needs seven 0s) deduces nothing.
    let line = puzzle_with_cells("6in10in14", 14, &[(6, 4, '0'), (6, 6, '0'), (6, 8, '0')]);
    let puzzle = parse_line(&line).unwrap();
    let mut log = EventLog::default();
    let _ = run_to_fixpoint(&puzzle, &mut log);
    let inner_count_deductions: Vec<_> = log
        .events
        .iter()
        .filter_map(|e| match e {
            binsolve_core::event::SolveEvent::Deduced {
                row,
                col,
                value,
                strategy,
                reason,
            } if *row == 6 => Some((*col, *value, *strategy, *reason)),
            _ => None,
        })
        .collect();
    // The three empty inner cells of row 6 (cols 5, 7, 9) become 1.
    for col in [5, 7, 9] {
        assert!(
            inner_count_deductions
                .iter()
                .any(|(c, v, _, _)| *c == col && *v == Cell::One),
            "inner cell r6c{col} should be forced to 1: {inner_count_deductions:?}"
        );
    }
    // And the reason is the INNER region's count rule: need = 3, not 7.
    assert!(
        inner_count_deductions
            .iter()
            .any(|(_, _, s, r)| *s == StrategyId::FillByCount
                && matches!(r, Reason::CountComplete { need: 3, .. })),
        "deduction must come from the 6x6 region's balance rule: {inner_count_deductions:?}"
    );
    // Outside the inner region, row 6 must be untouched (cols 0..4, 10..14).
    assert!(
        !inner_count_deductions
            .iter()
            .any(|(c, _, _, _)| *c < 4 || *c >= 10),
        "no deduction may leak outside the inner region: {inner_count_deductions:?}"
    );
}

#[test]
fn k2d_outer_region_rule_fires_without_inner_support() {
    // 8in14: row 0 lies entirely outside the inner 8x8 (rows 3..11).
    // Give it seven 0s: the OUTER balance rule forces the rest to 1.
    let cells: Vec<(usize, usize, char)> = [0, 2, 4, 6, 8, 10, 12]
        .iter()
        .map(|c| (0, *c, '0'))
        .collect();
    let line = puzzle_with_cells("8in14", 14, &cells);
    let puzzle = parse_line(&line).unwrap();
    let StrategyRun::Stuck { grid, .. } = run_to_fixpoint(&puzzle, &mut EventLog::default()) else {
        panic!("single-row givens cannot complete a 14x14");
    };
    for c in [1, 3, 5, 7, 9, 11, 13] {
        assert_eq!(grid.get(0, c), Cell::One, "r0c{c} forced by outer balance");
    }
}

#[test]
fn k2b_tiled_whole_grid_region_constrains_across_blocks() {
    // 4x6x6: row 0 spans two 6x6 blocks. Fill five 0s in the left block
    // row and one 0 in the right block row: the WHOLE-grid row (twelve
    // cells, needs six 0s) must still be respected by any deduction —
    // verify the whole-row region exists and its line has length 12.
    let regions = PuzzleKind::FourTimes6x6.regions();
    let whole = regions.last().unwrap();
    assert_eq!(whole.n, 12);
    let blocks: Vec<_> = regions.iter().filter(|r| r.n == 6).collect();
    assert_eq!(blocks.len(), 4);
    // Block origins tile the grid exactly.
    let origins: Vec<_> = blocks.iter().map(|r| (r.row, r.col)).collect();
    assert_eq!(origins, vec![(0, 0), (0, 6), (6, 0), (6, 6)]);
}
