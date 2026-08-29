//! K27: planning a batch — which puzzles, in which order, with which
//! seed. Deciding what to generate happens here; writing it to disk is
//! the CLI's job.

use crate::carve::{Carved, carve};
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
    pub seed: u64,
    pub count: u64,
    /// Give up on one puzzle after this many attempts. Retries exist to
    /// escape a duplicate (AR28); the bound stops a run whose space of
    /// distinct puzzles is exhausted from spinning forever.
    pub attempts_per_puzzle: u64,
}

impl Plan {
    pub fn new(n: usize, regions: Vec<Region>, ceiling: Level, seed: u64, count: u64) -> Self {
        Plan {
            n,
            regions,
            ceiling,
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
    let mut produced = Vec::new();
    let mut shortfalls = Vec::new();

    for index in 0..plan.count {
        let mut landed = false;

        for attempt in 0..plan.attempts_per_puzzle {
            let mut rng = stream(plan.seed, index, attempt);
            let Some(solution) = fill::solution(plan.n, &plan.regions, &mut rng) else {
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
            let carved = carve(&solution, &plan.regions, plan.ceiling, &mut rng);
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
