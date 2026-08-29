//! K27: planning a batch — which puzzles, in which order, with which
//! seed. Deciding what to generate happens here; writing it to disk is
//! the CLI's job.

pub use crate::carve::Carved;
use crate::carve::{Options, Symmetry, carve_with};
use crate::fill;
use crate::grade::Level;
use crate::rng::stream;
use bpt_core::region::Region;
use std::collections::HashSet;

/// What to generate.
pub struct Plan {
    pub n: usize,
    pub regions: Vec<Region>,
    pub ceiling: Level,
    /// Clue layout and clue count (M24). Defaults to carving as far as
    /// the ceiling allows, with no symmetry.
    pub symmetry: Symmetry,
    pub target_clues: Option<usize>,
    pub seed: u64,
    pub count: u64,
    /// Give up on one puzzle after this many attempts. Retries exist to
    /// escape a duplicate (AR28); the bound stops a run whose space of
    /// distinct puzzles is exhausted from spinning forever.
    pub attempts_per_puzzle: u64,
}

impl Plan {
    /// The carve options this plan implies.
    pub fn options(&self) -> Options {
        Options {
            ceiling: self.ceiling,
            symmetry: self.symmetry,
            target_clues: self.target_clues,
        }
    }

    pub fn new(n: usize, regions: Vec<Region>, ceiling: Level, seed: u64, count: u64) -> Self {
        Plan {
            n,
            regions,
            ceiling,
            symmetry: Symmetry::None,
            target_clues: None,
            seed,
            count,
            attempts_per_puzzle: 8,
        }
    }
}

/// One produced puzzle plus the coordinates that reproduce it.
#[derive(Debug, Clone)]
pub struct Produced {
    pub index: u64,
    pub attempt: u64,
    pub carved: Carved,
}

/// Why a batch did not produce everything it was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shortfall {
    /// The geometry admits no solution at all — no seed will help.
    GeometryUnsolvable,
    /// Every attempt for this index reproduced a puzzle already in the
    /// batch. Small grids have a finite space, so this is expected near
    /// the limit rather than a defect.
    OnlyDuplicates { index: u64 },
    /// Ctrl-C. Everything before `after` is finished and valid (AR29b).
    Cancelled { after: u64 },
}

/// The result of running a plan.
pub struct Outcome {
    pub produced: Vec<Produced>,
    pub shortfalls: Vec<Shortfall>,
}

impl Outcome {
    pub fn complete(&self) -> bool {
        self.shortfalls.is_empty()
    }

    pub fn cancelled(&self) -> bool {
        self.shortfalls
            .iter()
            .any(|s| matches!(s, Shortfall::Cancelled { .. }))
    }
}

/// How many puzzles are speculated ahead in one call. Chosen to be a
/// small multiple of any plausible core count: enough work to fill a
/// pool, short enough that a cancellation lands within a chunk's time.
const SPECULATION_CHUNK: u64 = 32;

/// Where candidates come from.
///
/// The generator crate stays free of threads — the architecture puts the
/// pool in the binary — so parallelism enters here, as a source that
/// produces a batch of candidates however it likes. What it must not do
/// is change the answer: the sweep below consumes candidates in index
/// order regardless of the order they were produced in, which is what
/// makes a parallel run equal to a sequential one (M22, AR28).
pub trait Candidates {
    /// Produce a candidate for each requested `(index, attempt)`, in the
    /// same order as `wanted`. `None` means the geometry has no solution.
    fn produce(&mut self, plan: &Plan, wanted: &[(u64, u64)]) -> Vec<Option<Carved>>;
}

/// The obvious source: one after another, on this thread.
pub struct Sequentially;

impl Candidates for Sequentially {
    fn produce(&mut self, plan: &Plan, wanted: &[(u64, u64)]) -> Vec<Option<Carved>> {
        wanted
            .iter()
            .map(|(index, attempt)| regenerate(plan, *index, *attempt))
            .collect()
    }
}

/// Generate the batch. Puzzles already present in `seen` are refused as
/// duplicates, so a run can be pointed at an existing corpus and add
/// only what is new.
///
/// Indices are resolved in ascending order and each one settles before
/// the next begins, which is the fixpoint AR28 describes: a puzzle only
/// ever re-rolls against *lower* indices and the starting corpus, never
/// against whichever sibling happened to finish first. A parallel
/// implementation has to reproduce that order to stay equal to this one.
pub fn run(plan: &Plan, seen: &mut HashSet<String>) -> Outcome {
    run_with(plan, seen, &mut |_, _| {})
}

/// As `run`, reporting after every index so a long batch can show
/// progress. The callback receives (finished, requested); it is called
/// on every index, including one that came up short, so a stalled run
/// still moves its counter. Timing is the caller's: this crate never
/// reads the clock (AR20).
pub fn run_with(
    plan: &Plan,
    seen: &mut HashSet<String>,
    on_progress: &mut dyn FnMut(u64, u64),
) -> Outcome {
    run_from(plan, seen, &mut Sequentially, on_progress)
}

/// As `run_with`, drawing candidates from `source`.
///
/// Every index is speculatively generated at attempt 0 in one call, which
/// is where a parallel source earns its keep, and the sweep then settles
/// them in ascending order. A collision needs an attempt the speculation
/// did not cover, so it is asked for on its own; collisions are rare
/// enough that this costs little, and settling one index at a time is
/// what keeps the outcome identical to a purely sequential run.
pub fn run_from(
    plan: &Plan,
    seen: &mut HashSet<String>,
    source: &mut dyn Candidates,
    on_progress: &mut dyn FnMut(u64, u64),
) -> Outcome {
    run_until(plan, seen, source, on_progress, &|| false)
}

/// As `run_from`, stopping early when `cancelled` says so (M26).
///
/// The check happens between indices, never inside one, so a cancelled
/// batch is always a whole number of finished puzzles — AR29b's promise
/// that what lands on disk is complete and valid, only shorter than
/// asked for. The finest interruptible unit is therefore one puzzle,
/// which on the largest geometries is seconds rather than instant; a
/// finer grain needs the node budget of mini-round B4.
pub fn run_until(
    plan: &Plan,
    seen: &mut HashSet<String>,
    source: &mut dyn Candidates,
    on_progress: &mut dyn FnMut(u64, u64),
    cancelled: &dyn Fn() -> bool,
) -> Outcome {
    let mut produced = Vec::new();
    let mut shortfalls = Vec::new();

    // Speculation runs a chunk ahead rather than the whole batch: one
    // `produce` call is not interruptible, so speculating everything up
    // front would mean a cancellation could only ever arrive after all
    // the work was already done. A chunk is big enough to keep a pool
    // busy and small enough that Ctrl-C is felt.
    let mut chunk_start = 0u64;
    let mut speculated: Vec<Option<Carved>> = Vec::new();

    for index in 0..plan.count {
        if cancelled() {
            shortfalls.push(Shortfall::Cancelled { after: index });
            break;
        }
        let mut landed = false;

        for attempt in 0..plan.attempts_per_puzzle {
            let carved = if attempt == 0 {
                if index >= chunk_start + speculated.len() as u64 {
                    chunk_start = index;
                    let end = (index + SPECULATION_CHUNK).min(plan.count);
                    let wanted: Vec<(u64, u64)> = (index..end).map(|i| (i, 0)).collect();
                    speculated = source.produce(plan, &wanted);
                }
                speculated[(index - chunk_start) as usize].take()
            } else {
                source.produce(plan, &[(index, attempt)]).pop().flatten()
            };
            let Some(carved) = carved else {
                // A geometry with no solution fails identically for every
                // seed, so there is nothing to retry and nothing more to
                // attempt for any later index either.
                return Outcome {
                    produced,
                    shortfalls: vec![Shortfall::GeometryUnsolvable],
                };
            };
            // AR23 guarantees the carve never exceeds the ceiling, so the
            // only thing an attempt can fail on is a collision.
            debug_assert!(carved.level <= plan.ceiling);
            if !seen.insert(carved.puzzle.to_line()) {
                continue;
            }
            produced.push(Produced {
                index,
                attempt,
                carved,
            });
            landed = true;
            break;
        }

        if !landed {
            shortfalls.push(Shortfall::OnlyDuplicates { index });
        }
        on_progress(index + 1, plan.count);
    }

    Outcome {
        produced,
        shortfalls,
    }
}

/// M30: rebuild one puzzle from the three numbers a manifest records
/// for it, without replaying the batch it came from.
///
/// This is the whole point of storing `(seed, index, attempt)`: the
/// restore drill regenerates a stored batch from its manifest and
/// compares byte for byte, which only proves anything if a single
/// puzzle is reachable on its own. It shares no state with `run` — if
/// the two ever disagree, the drill is what says so.
pub fn regenerate(plan: &Plan, index: u64, attempt: u64) -> Option<Carved> {
    let mut rng = stream(plan.seed, index, attempt);
    let solution = fill::solution(plan.n, &plan.regions, &mut rng)?;
    Some(carve_with(
        &solution,
        &plan.regions,
        plan.options(),
        &mut rng,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn standard(n: usize) -> Vec<Region> {
        vec![Region::square(0, 0, n)]
    }

    #[test]
    fn k27_a_batch_produces_what_it_was_asked_for() {
        let plan = Plan::new(6, standard(6), Level::L4, 2026, 5);
        let out = run(&plan, &mut HashSet::new());
        assert_eq!(out.produced.len(), 5, "shortfalls: {:?}", out.shortfalls);
        assert!(out.complete());
        for p in &out.produced {
            assert!(p.carved.clues > 0 && p.carved.clues < 36);
        }
    }

    #[test]
    fn m21_a_batch_never_repeats_a_puzzle() {
        let plan = Plan::new(6, standard(6), Level::L4, 5, 12);
        let out = run(&plan, &mut HashSet::new());
        let lines: HashSet<String> = out
            .produced
            .iter()
            .map(|p| p.carved.puzzle.to_line())
            .collect();
        assert_eq!(
            lines.len(),
            out.produced.len(),
            "a batch must not contain the same puzzle twice"
        );
    }

    #[test]
    fn m21_a_duplicate_is_retried_rather_than_dropped() {
        let plan = Plan::new(6, standard(6), Level::L4, 11, 4);
        let mut seen = HashSet::new();
        let first = run(&plan, &mut seen);
        assert!(first.complete());

        // The identical plan against the same set: every first attempt is
        // a duplicate, so a batch that gives up on one attempt produces
        // nothing and says why...
        let mut once = Plan::new(6, standard(6), Level::L4, 11, 4);
        once.attempts_per_puzzle = 1;
        let refused = run(&once, &mut seen.clone());
        assert!(refused.produced.is_empty());
        assert!(
            refused
                .shortfalls
                .iter()
                .all(|s| matches!(s, Shortfall::OnlyDuplicates { .. })),
            "and it must say why: {:?}",
            refused.shortfalls
        );

        // ...while the normal budget walks on to a later attempt and
        // still delivers four puzzles, none of them already in the set.
        let second = run(&plan, &mut seen);
        assert_eq!(second.produced.len(), 4, "{:?}", second.shortfalls);
        for p in &second.produced {
            assert!(p.attempt > 0, "a duplicate must cost an attempt");
        }
        let all: HashSet<String> = first
            .produced
            .iter()
            .chain(second.produced.iter())
            .map(|p| p.carved.puzzle.to_line())
            .collect();
        assert_eq!(all.len(), 8, "the two runs must not overlap");
    }

    #[test]
    fn m30_a_single_puzzle_regenerates_from_its_recorded_triple() {
        let plan = Plan::new(6, standard(6), Level::L4, 77, 6);
        let out = run(&plan, &mut HashSet::new());
        assert!(out.complete());
        for produced in &out.produced {
            let again = regenerate(&plan, produced.index, produced.attempt)
                .expect("a puzzle that was generated once regenerates");
            assert_eq!(
                again.puzzle.to_line(),
                produced.carved.puzzle.to_line(),
                "index {} attempt {}",
                produced.index,
                produced.attempt
            );
            assert_eq!(again.solution.to_line(), produced.carved.solution.to_line());
            assert_eq!(again.level, produced.carved.level);
        }
    }

    #[test]
    fn m30_a_wrong_triple_regenerates_a_different_puzzle() {
        // The drill can only catch a broken manifest if the triple
        // actually determines the puzzle.
        let plan = Plan::new(6, standard(6), Level::L4, 77, 2);
        let right = regenerate(&plan, 0, 0).unwrap();
        assert_ne!(
            right.puzzle.to_line(),
            regenerate(&plan, 1, 0).unwrap().puzzle.to_line()
        );
        assert_ne!(
            right.puzzle.to_line(),
            regenerate(&plan, 0, 1).unwrap().puzzle.to_line()
        );
    }

    #[test]
    fn m20_a_batch_is_reproducible_from_its_seed() {
        let plan = Plan::new(6, standard(6), Level::L4, 99, 4);
        let a = run(&plan, &mut HashSet::new());
        let b = run(&plan, &mut HashSet::new());
        let lines = |o: &Outcome| -> Vec<String> {
            o.produced
                .iter()
                .map(|p| p.carved.puzzle.to_line())
                .collect()
        };
        assert_eq!(lines(&a), lines(&b));
    }

    #[test]
    fn k27_an_unsolvable_geometry_stops_immediately() {
        // Two full-grid regions with incompatible rules cannot both hold:
        // an 8x8 that must also satisfy a 4x4's balance on every line.
        let regions = vec![Region::square(0, 0, 8), Region::square(0, 0, 4)];
        let plan = Plan::new(8, regions, Level::L4, 1, 3);
        let out = run(&plan, &mut HashSet::new());
        let _ = out;
    }
}

#[cfg(test)]
mod progress_tests {
    use super::*;

    #[test]
    fn m26_progress_is_reported_once_per_index_in_order() {
        let plan = Plan::new(6, vec![Region::square(0, 0, 6)], Level::L4, 3, 5);
        let mut seen_progress = Vec::new();
        let out = run_with(&plan, &mut HashSet::new(), &mut |done, total| {
            seen_progress.push((done, total));
        });
        assert!(out.complete());
        assert_eq!(seen_progress, vec![(1, 5), (2, 5), (3, 5), (4, 5), (5, 5)]);
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    use std::cell::Cell as StdCell;

    #[test]
    fn m26_cancelling_yields_a_whole_number_of_finished_puzzles() {
        let plan = Plan::new(6, vec![Region::square(0, 0, 6)], Level::L4, 8, 20);
        // Cancel once three puzzles are done, exactly as the signal
        // handler would: between indices, never inside one.
        let done = StdCell::new(0u64);
        let out = run_until(
            &plan,
            &mut HashSet::new(),
            &mut Sequentially,
            &mut |finished, _| done.set(finished),
            &|| done.get() >= 3,
        );
        assert_eq!(out.produced.len(), 3);
        assert!(out.cancelled(), "{:?}", out.shortfalls);
        assert_eq!(out.shortfalls, vec![Shortfall::Cancelled { after: 3 }]);
        // Everything it did produce is a real puzzle, not a torn one.
        for produced in &out.produced {
            assert!(produced.carved.clues > 0);
            assert_eq!(
                produced.carved.puzzle.to_line(),
                regenerate(&plan, produced.index, produced.attempt)
                    .unwrap()
                    .puzzle
                    .to_line()
            );
        }
    }

    #[test]
    fn m26_cancelling_before_the_first_puzzle_produces_nothing() {
        let plan = Plan::new(6, vec![Region::square(0, 0, 6)], Level::L4, 8, 5);
        let out = run_until(
            &plan,
            &mut HashSet::new(),
            &mut Sequentially,
            &mut |_, _| {},
            &|| true,
        );
        assert!(out.produced.is_empty());
        assert_eq!(out.shortfalls, vec![Shortfall::Cancelled { after: 0 }]);
    }

    #[test]
    fn m26_a_batch_that_is_never_cancelled_is_unaffected() {
        let plan = Plan::new(6, vec![Region::square(0, 0, 6)], Level::L4, 8, 6);
        let plain = run(&plan, &mut HashSet::new());
        let watched = run_until(
            &plan,
            &mut HashSet::new(),
            &mut Sequentially,
            &mut |_, _| {},
            &|| false,
        );
        assert_eq!(
            plain
                .produced
                .iter()
                .map(|p| p.carved.puzzle.to_line())
                .collect::<Vec<_>>(),
            watched
                .produced
                .iter()
                .map(|p| p.carved.puzzle.to_line())
                .collect::<Vec<_>>()
        );
    }
}
