//! L5 region-boundary tests [K2a-K2e]: deductions must respect region
//! boundaries — an inner-region rule fires without outer-grid support
//! and vice versa. Per-type solve coverage lives in the corpus-driven
//! suites (sweep, solve, corpus).

use bpt_core::event::EventLog;
use bpt_core::grid::Cell;
use bpt_core::parse::parse_line;
use bpt_core::region::PuzzleKind;
use bpt_core::search::{StrategyRun, run_to_fixpoint};
use bpt_core::strategy::{Reason, StrategyId};

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
            bpt_core::event::SolveEvent::Deduced {
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

/// K2a/K2b/K2c promised a boundary deduction per tiled type. The old
/// test asserted only region shapes, duplicating a check in region.rs.
/// The deduction that ONLY the whole-grid region can make is across a
/// block seam: cells 5 and 6 of a row sit in different blocks, so no
/// block rule ever sees them as neighbours.
#[test]
fn k2a_whole_grid_no_triple_fires_across_the_block_seam() {
    // 4x6x6 = 12x12 of four 6x6 blocks. r0c4 and r0c5 are the last two
    // cells of the left block; r0c6 is the first of the right block.
    // Two zeros ending the left block force a 1 just past the seam —
    // a rule only the 12-wide row has.
    let line = puzzle_with_cells("4x6x6", 12, &[(0, 4, '0'), (0, 5, '0')]);
    let puzzle = parse_line(&line).unwrap();
    let mut log = EventLog::default();
    let _ = run_to_fixpoint(&puzzle, &mut log);

    let across_seam = log.events.iter().any(|e| {
        matches!(
            e,
            bpt_core::event::SolveEvent::Deduced {
                row: 0,
                col: 6,
                value: Cell::One,
                ..
            }
        )
    });
    assert!(
        across_seam,
        "r0c6 must be forced to 1 by the whole-grid row; only that region \
         sees r0c5 and r0c6 as neighbours. Events: {:?}",
        log.events
    );
}

/// The mirror case: a deduction only a BLOCK can make. Three zeros in
/// the left block's row fill that block's quota (6/2 = 3), so its
/// remaining cells must be 1 — while the 12-wide row still allows three
/// more zeros and therefore deduces nothing.
#[test]
fn k2a_block_count_rule_fires_where_the_whole_row_is_silent() {
    let line = puzzle_with_cells("4x6x6", 12, &[(0, 0, '0'), (0, 2, '0'), (0, 4, '0')]);
    let puzzle = parse_line(&line).unwrap();
    let mut log = EventLog::default();
    let _ = run_to_fixpoint(&puzzle, &mut log);

    let block_fills: Vec<_> = log
        .events
        .iter()
        .filter_map(|e| match e {
            bpt_core::event::SolveEvent::Deduced {
                row: 0,
                col,
                value,
                strategy,
                reason,
            } => Some((*col, *value, *strategy, *reason)),
            _ => None,
        })
        .collect();

    for col in [1, 3, 5] {
        assert!(
            block_fills
                .iter()
                .any(|(c, v, _, _)| *c == col && *v == Cell::One),
            "r0c{col} must be forced to 1 by the block's count rule: {block_fills:?}"
        );
    }
    // And the reason must be the BLOCK's quota (three), not the row's six.
    assert!(
        block_fills
            .iter()
            .any(|(_, _, s, r)| *s == StrategyId::FillByCount
                && matches!(r, Reason::CountComplete { need: 3, .. })),
        "the deduction must come from the 6x6 block, need = 3: {block_fills:?}"
    );
}

#[test]
fn k2c_nine_block_layout_tiles_the_grid_exactly() {
    let regions = PuzzleKind::NineTimes6x6.regions();
    assert_eq!(regions.len(), 10, "nine blocks plus the whole grid");
    let whole = regions.last().unwrap();
    assert_eq!((whole.rows, whole.cols), (18, 18));
    let origins: Vec<_> = regions
        .iter()
        .filter(|r| r.rows == 6)
        .map(|r| (r.row, r.col))
        .collect();
    assert_eq!(
        origins,
        vec![
            (0, 0),
            (0, 6),
            (0, 12),
            (6, 0),
            (6, 6),
            (6, 12),
            (12, 0),
            (12, 6),
            (12, 12)
        ]
    );
}
