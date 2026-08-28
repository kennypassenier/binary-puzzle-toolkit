//! Generator core for binary puzzles (Takuzu/Binairo), covering standard
//! grids, the five composite types from binarypuzzle.com, and invented
//! geometries. Pure logic: no file access, no network, no threads and no
//! clock reads (AR1) — the CLI owns all of that. See
//! docs/ARCHITECTURE_DECISIONS.md.

#![forbid(unsafe_code)]

pub mod batch;
pub mod carve;
pub mod error;
pub mod fill;
pub mod geometry;
pub mod grade;
pub mod rng;

#[cfg(test)]
mod tests {
    #[test]
    fn skeleton_compiles_and_tests_run() {
        // L0 walking-skeleton smoke test; replaced by real suites in L1+.
        assert_eq!(2 + 2, 4);
    }
}
