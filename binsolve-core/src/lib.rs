//! Solver core for binary puzzles (Takuzu/Binairo), including the five
//! composite types from binarypuzzle.com. Pure logic: no I/O, no
//! dependencies (AR1). See docs/ARCHITECTURE_DECISIONS.md.

#![forbid(unsafe_code)]

pub mod event;
pub mod grid;
pub mod parse;
pub mod region;
pub mod search;
pub mod strategy;

#[cfg(test)]
mod tests {
    #[test]
    fn skeleton_compiles_and_tests_run() {
        // L0 walking-skeleton smoke test; replaced by real suites in L1+.
        assert_eq!(2 + 2, 4);
    }
}
