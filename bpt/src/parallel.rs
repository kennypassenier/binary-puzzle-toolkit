//! M22: generate a batch across cores.
//!
//! The pool lives in the binary because the generator crate is
//! deliberately free of threads. All this adds is *where* candidates are
//! computed; which of them end up in the batch, and in what order, is
//! decided by the sweep in `bpt_forge::batch`, so a parallel run and a
//! sequential run produce the same puzzles for the same seed.

use bpt_forge::batch::{self, Candidates, Carved, Plan};
use rayon::prelude::*;

/// Candidates computed on the rayon pool, collected back into the order
/// they were asked for. Collecting by input order rather than completion
/// order is the whole trick (T7): nothing downstream can observe which
/// worker finished first.
pub struct OnAllCores;

impl Candidates for OnAllCores {
    fn produce(&mut self, plan: &Plan, wanted: &[(u64, u64)]) -> Vec<Option<Carved>> {
        wanted
            .par_iter()
            .map(|(index, attempt)| batch::regenerate(plan, *index, *attempt))
            .collect()
    }
}
