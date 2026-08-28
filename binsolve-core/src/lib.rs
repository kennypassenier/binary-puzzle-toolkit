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
