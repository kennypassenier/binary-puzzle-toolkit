//! Generator for binary puzzles (Takuzu/Binairo): standard grids, the
//! five composite types from binarypuzzle.com, and invented geometries
//! read from configuration. Pure logic — no file access, no network, no
//! threads, no clock reads; the CLI owns all of that.
//!
//! Geometry is defined here rather than in `bpt-core` because it is a
//! generator concern: the solver receives a puzzle whose regions are
//! already fixed, while the generator has to construct and validate
//! them. Both sides speak `bpt_core::region::Region`.

#![forbid(unsafe_code)]

pub mod batch;
pub mod carve;
pub mod error;
pub mod fill;
pub mod geometry;
pub mod grade;
pub mod inspect;
pub mod rng;
