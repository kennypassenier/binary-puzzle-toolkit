//! K21/K25: remove clues from a full solution, proving uniqueness after
//! each removal, and restore-and-lock any removal that pushes the puzzle
//! past the requested difficulty (AR23).
//!
//! Why restore-and-lock rather than carve-to-minimal-then-repair: on
//! measured data, 23 of 28 minimal puzzles were unsolvable by any tier
//! without guessing — outside the scale entirely — and restoring clues
//! does not monotonically lower difficulty, so a repair loop can thrash
//! and then report a level unreachable that in fact is not. Keeping a
//! ceiling converges in one pass and never restarts.

use crate::grade::{Level, level_of_solvable};
use bpt_core::event::NullObserver;
use bpt_core::grid::{Cell, Grid};
use bpt_core::region::{Puzzle, Region};
use bpt_core::search::{SolveMode, SolveOutcome, solve, solve_within};
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

/// What a carve produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carved {
    pub puzzle: Grid,
    pub solution: Grid,
    pub level: Level,
    /// Cells still filled in the puzzle.
    pub clues: usize,
    /// How many removals were refused because the uniqueness question
    /// ran out of budget rather than being answered (B4).
    ///
    /// Recorded rather than swallowed: a puzzle with hits carries clues
    /// it might not have needed, and nobody should have to guess whether
    /// that happened. Zero is the normal case.
    pub budget_hits: usize,
}

/// B4/AR26: how many search nodes one uniqueness question may cost.
///
/// Every removal asks "does the other value lead anywhere?", and on the
/// largest grids that question sometimes has no cheap answer — measured,
/// a single 18x18 ran for over eighty minutes without finishing. The
/// budget makes the cost of a carve bounded by construction.
///
/// The number was chosen by measuring what it costs. Against a budget
/// of fifty million — effectively unbounded — 200 000 changes almost
/// nothing:
///
/// | geometry | fired on | total hits | extra clues |
/// |---|---|---|---|
/// | 12x12 | 0 of 20 puzzles | 0 | 0 |
/// | 14x14 | 0 of 10 | 0 | 0 |
/// | 8in14 | 1 of 10 | 4 | 1 |
/// | 16x16 | 2 of 3 | 3 | 0 |
///
/// So it is invisible below 16x16, and where it does fire it costs at
/// most a single clue — while taking 8in14's p95 from 41 s to 18 s and
/// turning 18x18 seeds that never finished into ones that take under
/// two minutes. `Carved::budget_hits` records every firing, so a puzzle
/// that paid this price says so.
pub const UNIQUENESS_BUDGET: u64 = 200_000;

/// How clues may be laid out (M24).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Symmetry {
    /// No constraint: every cell is removed on its own.
    #[default]
    None,
    /// A clue at (r, c) implies one at (n-1-r, n-1-c) — the pattern a
    /// half turn leaves unchanged.
    Rotational,
    /// A clue at (r, c) implies one at (r, n-1-c) — mirrored left to
    /// right.
    Mirror,
}

impl Symmetry {
    /// The cells that must be removed together with (row, col) — always
    /// including the cell itself, deduplicated, so a cell on the axis
    /// gives a group of one.
    fn orbit(self, n: usize, row: usize, col: usize) -> Vec<(usize, usize)> {
        let partner = match self {
            Symmetry::None => return vec![(row, col)],
            Symmetry::Rotational => (n - 1 - row, n - 1 - col),
            Symmetry::Mirror => (row, n - 1 - col),
        };
        if partner == (row, col) {
            vec![(row, col)]
        } else {
            vec![(row, col), partner]
        }
    }
}

/// What to aim for (M24).
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Hardest level the result may need.
    pub ceiling: Level,
    /// Layout constraint on the clues.
    pub symmetry: Symmetry,
    /// Stop once this many clues remain. `None` carves as far as it can,
    /// which is the default and what a batch wants.
    pub target_clues: Option<usize>,
    /// B4: search nodes one uniqueness question may cost.
    pub budget: u64,
}

impl Options {
    pub fn new(ceiling: Level) -> Self {
        Options {
            ceiling,
            symmetry: Symmetry::None,
            target_clues: None,
            budget: UNIQUENESS_BUDGET,
        }
    }
}

/// Carve `solution` down to a puzzle that is uniquely solvable and no
/// harder than `ceiling`.
///
/// Every intermediate state is proven unique before it is accepted, so
/// the result carries the same guarantee as a published puzzle: exactly
/// one solution, verifiable without an answer key.
pub fn carve(solution: &Grid, regions: &[Region], ceiling: Level, rng: &mut ChaCha8Rng) -> Carved {
    carve_with(solution, regions, Options::new(ceiling), rng)
}

/// Carve under `options` (M24).
///
/// The loop always removes a *group*; without symmetry every group holds
/// one cell, which is the ordinary case and the fast one (AR31).
pub fn carve_with(
    solution: &Grid,
    regions: &[Region],
    options: Options,
    rng: &mut ChaCha8Rng,
) -> Carved {
    let n = solution.size();
    let mut working = solution.clone();

    let mut order: Vec<(usize, usize)> = (0..n).flat_map(|r| (0..n).map(move |c| (r, c))).collect();
    order.shuffle(rng);

    let mut budget_hits = 0usize;
    for (row, col) in order {
        if let Some(target) = options.target_clues {
            // Asking for a clue count means asking to stop, not to keep
            // going and hope. Carving further would only walk past it.
            if working.filled_count() <= target {
                break;
            }
        }
        let group = options.symmetry.orbit(n, row, col);
        let removed: Vec<Cell> = group.iter().map(|(r, c)| working.get(*r, *c)).collect();
        if removed.iter().any(|cell| cell.is_empty()) {
            continue;
        }
        for (r, c) in &group {
            working.set(*r, *c, Cell::Empty);
        }

        // At L4 the ceiling cannot reject anything — every candidate is
        // solvable by construction and L4 is the top of the scale — so
        // the ladder is not run at all in the default case. Below L4 it
        // decides, and it is the only thing that has to.
        let verdict = still_unique(&working, regions, &group, &removed, options.budget);
        if verdict == Uniqueness::Unknown {
            budget_hits += 1;
        }
        let acceptable = verdict == Uniqueness::Holds
            && (options.ceiling == Level::L4
                || fits_ceiling(
                    &Puzzle::custom(working.clone(), regions.to_vec()),
                    options.ceiling,
                ));
        // A removal that costs uniqueness, or that pushes the puzzle past
        // the requested ceiling, is put back and never tried again.
        if !acceptable {
            for ((r, c), cell) in group.iter().zip(&removed) {
                working.set(*r, *c, *cell);
            }
        }
    }

    let puzzle = Puzzle::custom(working.clone(), regions.to_vec());
    let measured = level_of_solvable(&puzzle);
    Carved {
        clues: working.filled_count(),
        puzzle: working,
        solution: solution.clone(),
        level: measured,
        budget_hits,
    }
}

/// Does the puzzle still have exactly one solution now that `group` is
/// empty?
///
/// A single removed cell takes the cheap route below. A group of two
/// cannot: the argument there rests on there being exactly one cell
/// whose value is in question, so a symmetric carve pays for a full
/// uniqueness proof per group. That is the cost of symmetry, and M24
/// says so.
/// What the uniqueness question answered. `Unknown` is not a third kind
/// of "no": the removal is refused either way, but only `Unknown` means
/// nothing was actually proved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Uniqueness {
    Holds,
    Broken,
    Unknown,
}

fn still_unique(
    working: &Grid,
    regions: &[Region],
    group: &[(usize, usize)],
    removed: &[Cell],
    budget: u64,
) -> Uniqueness {
    match group {
        [(row, col)] => still_unique_without(working, regions, *row, *col, removed[0], budget),
        _ => match solve_within(
            &Puzzle::custom(working.clone(), regions.to_vec()),
            SolveMode::ProveUniqueness,
            &mut NullObserver,
            budget,
        ) {
            SolveOutcome::Solved { .. } => Uniqueness::Holds,
            SolveOutcome::BudgetExhausted { .. } => Uniqueness::Unknown,
            _ => Uniqueness::Broken,
        },
    }
}

/// Is this candidate at most as hard as `ceiling`?
///
/// `level` answers a harder question than the loop asks. Its expensive
/// half is the fallback that separates "needs guessing" from "has no
/// solution at all" by running a full search — and inside carve that
/// distinction is never in doubt, because every candidate is solvable by
/// construction: the solution it was carved from still solves it.
///
/// So the ladder alone decides. It stalls exactly on the puzzles that
/// need guessing, which are L4, and those pass only when L4 is the
/// ceiling. Measured on a 16x16: with the search still in the loop one
/// carve took 11.7 s, without it 5.5 s, and the puzzles produced are
/// byte for byte the same — which the restore drill re-checks against a
/// batch generated before either change.
fn fits_ceiling(candidate: &Puzzle, ceiling: Level) -> bool {
    match solve(candidate, SolveMode::StrategiesOnly, &mut NullObserver) {
        SolveOutcome::Solved { stats, .. } => {
            let reached = match stats.max_tier {
                0..=2 => Level::L1,
                3 => Level::L2,
                _ => Level::L3,
            };
            reached <= ceiling
        }
        // Stalled: solvable but not without guessing, which is L4.
        SolveOutcome::Stuck { .. } => ceiling == Level::L4,
        // Cannot happen while the invariant holds; refusing is the safe
        // answer if it ever stops holding.
        _ => false,
    }
}

/// Does the puzzle still have exactly one solution now that (row, col)
/// is empty?
///
/// The direct reading — "prove the whole puzzle unique again" — is what
/// this used to do, and it is what made carving unusable above 14x14:
/// one 16x16 took 30.8 s against 130 ms at 14x14, because proving
/// uniqueness means exhausting a search tree that grows with every clue
/// removed.
///
/// The cheap reading is exactly equivalent. Before the removal the
/// puzzle had exactly one solution, in which this cell held `was`. Every
/// solution of the emptied puzzle therefore either holds `was` here — of
/// which there is precisely one, the original — or holds the opposite.
/// So uniqueness survives if and only if pinning the opposite value
/// yields no solution at all, and refuting a wrong value is fast: it
/// contradicts something nearby instead of exploring the whole space.
///
/// The invariant this rests on is carve's own: it starts from a complete
/// solution and never accepts a removal that breaks uniqueness, so the
/// pre-removal puzzle is unique at every step.
fn still_unique_without(
    working: &Grid,
    regions: &[Region],
    row: usize,
    col: usize,
    was: Cell,
    budget: u64,
) -> Uniqueness {
    let opposite = match was {
        Cell::Zero => Cell::One,
        Cell::One => Cell::Zero,
        // Only a filled cell is ever removed, so this cannot arise; if it
        // ever did, refusing the removal is the safe answer.
        Cell::Empty => return Uniqueness::Broken,
    };
    let mut probe = working.clone();
    probe.set(row, col, opposite);
    match solve_within(
        &Puzzle::custom(probe, regions.to_vec()),
        SolveMode::FirstSolution,
        &mut NullObserver,
        budget,
    ) {
        // Refuted: the other value leads nowhere, so uniqueness survives.
        SolveOutcome::Contradiction { .. } => Uniqueness::Holds,
        // The other value works too, so the removal cost uniqueness.
        SolveOutcome::Solved { .. } | SolveOutcome::MultipleSolutions { .. } => Uniqueness::Broken,
        // B4: the search ran out of budget, so nothing was proved. The
        // clue goes back — the safe direction — and the caller counts it.
        SolveOutcome::BudgetExhausted { .. } => Uniqueness::Unknown,
        SolveOutcome::Stuck { .. } => Uniqueness::Broken,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fill;
    use crate::rng::stream;
    use bpt_core::region::validate_givens;

    fn standard(n: usize) -> Vec<Region> {
        vec![Region::square(0, 0, n)]
    }

    fn carved_6x6(seed: u64, ceiling: Level) -> Carved {
        let regions = standard(6);
        let mut rng = stream(seed, 0, 0);
        let solution = fill::solution(6, &regions, &mut rng).expect("6x6 fills");
        carve(&solution, &regions, ceiling, &mut rng)
    }

    #[test]
    fn k21_a_carved_puzzle_is_uniquely_solvable() {
        let out = carved_6x6(1, Level::L4);
        let puzzle = Puzzle::custom(out.puzzle.clone(), standard(6));
        let outcome = solve(&puzzle, SolveMode::ProveUniqueness, &mut NullObserver);
        let SolveOutcome::Solved { solution, .. } = outcome else {
            panic!("a carved puzzle must have exactly one solution: {outcome:?}");
        };
        assert_eq!(
            solution, out.solution,
            "the unique solution must be the one it was carved from"
        );
    }

    #[test]
    fn k21_carving_removes_clues_but_keeps_the_givens_consistent() {
        let out = carved_6x6(2, Level::L4);
        assert!(
            out.clues < 36,
            "carving must remove something, kept all {} cells",
            out.clues
        );
        assert!(out.clues > 0, "carving must not empty the grid");
        assert!(
            validate_givens(&out.puzzle, &out.solution).is_empty(),
            "every remaining clue must match the solution it came from"
        );
    }

    #[test]
    fn k25_the_ceiling_is_respected() {
        // Asking for L1 must never hand back something that needs more.
        for seed in 1..4 {
            let out = carved_6x6(seed, Level::L1);
            assert!(
                out.level <= Level::L1,
                "seed {seed}: asked for at most L1, got {}",
                out.level.name()
            );
        }
    }

    #[test]
    fn k25_a_higher_ceiling_carves_further() {
        // Stated over seeds rather than per seed on purpose. Carving is
        // greedy, so it is not monotone in the ceiling: a removal that a
        // higher ceiling accepts changes the grid, and can block a later
        // removal the lower ceiling would have taken. A single seed can
        // therefore come out one clue heavier at L4 than at L1 without
        // anything being wrong. The direction only holds in aggregate.
        let seeds = 1..21;
        let (mut easy, mut free) = (0usize, 0usize);
        for seed in seeds.clone() {
            easy += carved_6x6(seed, Level::L1).clues;
            free += carved_6x6(seed, Level::L4).clues;
        }
        assert!(
            free < easy,
            "over {} seeds L4 kept {free} clues and L1 kept {easy} — \
             a higher ceiling must carve deeper on average",
            seeds.count()
        );
    }

    fn carved_with(seed: u64, options: Options) -> Carved {
        let regions = standard(8);
        let mut rng = crate::rng::stream(seed, 0, 0);
        let solution = crate::fill::solution(8, &regions, &mut rng).expect("solvable");
        carve_with(&solution, &regions, options, &mut rng)
    }

    #[test]
    fn m24_a_symmetric_carve_produces_a_symmetric_clue_pattern() {
        type Partner = fn(usize, usize, usize) -> (usize, usize);
        let rotational: Partner = |n, r, c| (n - 1 - r, n - 1 - c);
        let mirror: Partner = |n, r, c| (r, n - 1 - c);
        for (symmetry, partner) in [
            (Symmetry::Rotational, rotational),
            (Symmetry::Mirror, mirror),
        ] {
            let mut options = Options::new(Level::L4);
            options.symmetry = symmetry;
            let out = carved_with(4, options);
            for row in 0..8 {
                for col in 0..8 {
                    let (pr, pc) = partner(8, row, col);
                    assert_eq!(
                        out.puzzle.get(row, col).is_empty(),
                        out.puzzle.get(pr, pc).is_empty(),
                        "{symmetry:?}: ({row},{col}) and ({pr},{pc}) disagree"
                    );
                }
            }
            // Still a real puzzle, not just a pretty pattern.
            assert!(out.clues > 0 && out.clues < 64);
        }
    }

    #[test]
    fn m24_symmetry_costs_clues() {
        // Removing in pairs means a pair that breaks uniqueness keeps
        // both cells, so a symmetric puzzle carries more clues than a
        // free one. Stated over seeds: the loop is greedy, so a single
        // seed can go either way.
        let (mut free, mut symmetric) = (0, 0);
        for seed in 1..8 {
            free += carved_with(seed, Options::new(Level::L4)).clues;
            let mut options = Options::new(Level::L4);
            options.symmetry = Symmetry::Rotational;
            symmetric += carved_with(seed, options).clues;
        }
        assert!(
            symmetric > free,
            "symmetric kept {symmetric} clues over seven seeds, free kept {free}"
        );
    }

    #[test]
    fn m24_a_clue_target_stops_the_carve() {
        let mut options = Options::new(Level::L4);
        options.target_clues = Some(40);
        let out = carved_with(4, options);
        assert!(
            out.clues <= 40 + 1,
            "asked to stop at 40 clues, stopped at {}",
            out.clues
        );
        // And it is still a puzzle with one solution, not a truncated
        // grid: the ceiling and uniqueness rules applied throughout.
        assert!(out.clues > 40 - 8, "stopped far short: {}", out.clues);
    }

    #[test]
    fn m24_an_unreachable_clue_target_carves_as_far_as_it_can() {
        // Nothing can reach one clue. The carve must not spin or lie —
        // it stops where uniqueness stops it, and the caller compares.
        let mut options = Options::new(Level::L4);
        options.target_clues = Some(1);
        let out = carved_with(4, options);
        assert!(out.clues > 1, "one clue cannot determine an 8x8");
        assert_eq!(out.clues, carved_with(4, Options::new(Level::L4)).clues);
    }

    #[test]
    fn b4_a_carve_reports_whether_its_budget_ever_ran_out() {
        // On an 8x8 nothing is expensive enough to exhaust the budget,
        // so the honest report is zero. The value matters because it is
        // the difference between "minimal" and "minimal as far as we
        // were willing to look".
        let out = carved_with(4, Options::new(Level::L4));
        assert_eq!(out.budget_hits, 0);
    }

    #[test]
    fn b4_a_tiny_budget_keeps_more_clues_and_says_so() {
        // Forcing the budget to zero blocks the search but not the
        // deductions: the strategy ladder still refutes a wrong value
        // outright, without spending a single search node. That is why a
        // budget of zero still carves — measured on an 8x8, the ladder
        // alone accounts for most removals — and it is the reason the
        // real budget is invisible on small grids.
        let mut starved = Options::new(Level::L4);
        starved.budget = 0;
        let a = carved_with(4, starved);
        let b = carved_with(4, Options::new(Level::L4));

        assert!(
            a.budget_hits > 0,
            "a starved carve must report its refusals"
        );
        assert_eq!(b.budget_hits, 0, "the real budget is not reached on an 8x8");
        assert!(
            a.clues > b.clues,
            "a starved carve keeps more clues: {} against {}",
            a.clues,
            b.clues
        );
        // What it produces is still a real puzzle, not a truncated grid.
        assert!(a.clues < 64);
    }

    #[test]
    fn b4_the_budget_is_deterministic_not_wall_clock() {
        // AR26: the same puzzle and the same budget give the same answer
        // however fast the machine is. Running the identical carve twice
        // must agree on the puzzle AND on the number of hits.
        let mut options = Options::new(Level::L4);
        options.budget = 500;
        let a = carved_with(7, options);
        let b = carved_with(7, options);
        assert_eq!(a, b);
    }

    #[test]
    fn m20_carving_is_reproducible_from_its_seed() {
        assert_eq!(carved_6x6(42, Level::L4), carved_6x6(42, Level::L4));
    }
}
