//! Replay model and widget layout for the binsolve TUI [K15]. Exposed
//! as a library so the render model can be tested without a terminal
//! (AR9); the binary is a thin event loop over these modules.

#![forbid(unsafe_code)]

pub mod replay;
pub mod ui;
