//! The AR8 event stream: one spine feeding the trace (K16), stats
//! (K11), difficulty grading (M2) and the TUI (K15). Recording an
//! event log and replaying it later is the AR9 TUI model.

use crate::grid::Cell;
use crate::strategy::{Reason, StrategyId};
use std::fmt::Write as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveEvent {
    Deduced {
        row: usize,
        col: usize,
        value: Cell,
        strategy: StrategyId,
        reason: Reason,
    },
    /// L4: a search guess was made at `depth`.
    Guessed {
        row: usize,
        col: usize,
        value: Cell,
        depth: usize,
    },
    /// L4: the branch at `depth` was refuted; the guess cell reverts.
    Backtracked {
        to_depth: usize,
        row: usize,
        col: usize,
    },
    /// L4: a complete valid solution was reached (uniqueness search may
    /// continue past this point; stats count only up to the first one).
    SolutionFound,
}

pub trait Observer {
    fn on_event(&mut self, event: &SolveEvent);
}

/// The zero-cost path (AR8): used when nobody watches.
pub struct NullObserver;

impl Observer for NullObserver {
    fn on_event(&mut self, _event: &SolveEvent) {}
}

/// Records every event for later replay or formatting (AR9).
#[derive(Default)]
pub struct EventLog {
    pub events: Vec<SolveEvent>,
}

impl Observer for EventLog {
    fn on_event(&mut self, event: &SolveEvent) {
        self.events.push(*event);
    }
}

fn cell_name(row: usize, col: usize) -> String {
    format!("r{row}c{col}")
}

fn format_reason(reason: &Reason) -> String {
    match reason {
        Reason::AdjacentPair { a, b, value } => format!(
            "cells {} and {} are both {}, neighbours must differ",
            cell_name(a.0, a.1),
            cell_name(b.0, b.1),
            value.to_char()
        ),
        Reason::Gap { a, b, value } => format!(
            "cells {} and {} are both {}, the cell between must differ",
            cell_name(a.0, a.1),
            cell_name(b.0, b.1),
            value.to_char()
        ),
        Reason::CountComplete {
            is_row,
            line,
            value,
            need,
        } => format!(
            "{} {line} already has its {need} {}s, the rest get the opposite",
            if *is_row { "row" } else { "column" },
            value.to_char()
        ),
        Reason::UniqueSwap { is_row, other } => format!(
            "filling like {} {other} would duplicate it, so the open cells take the opposite values",
            if *is_row { "row" } else { "column" }
        ),
        Reason::CountInfeasible { tried } => format!(
            "a {} here leaves no valid way to complete the line",
            tried.to_char()
        ),
        Reason::Enumeration { survivors } => {
            format!("all {survivors} valid completions of this line agree on this cell")
        }
    }
}

/// Render an event log as the K16 numbered human-readable trace.
pub fn format_trace(events: &[SolveEvent]) -> String {
    let mut out = String::new();
    for (i, event) in events.iter().enumerate() {
        let step = i + 1;
        match event {
            SolveEvent::Deduced {
                row,
                col,
                value,
                strategy,
                reason,
            } => {
                let _ = writeln!(
                    out,
                    "step {step}: {} — {} = {} ({})",
                    strategy.name(),
                    cell_name(*row, *col),
                    value.to_char(),
                    format_reason(reason)
                );
            }
            SolveEvent::Guessed {
                row,
                col,
                value,
                depth,
            } => {
                let _ = writeln!(
                    out,
                    "step {step}: guess — {} = {} (strategies exhausted, depth {depth})",
                    cell_name(*row, *col),
                    value.to_char()
                );
            }
            SolveEvent::Backtracked { to_depth, row, col } => {
                let _ = writeln!(
                    out,
                    "step {step}: backtrack — {} contradicted, back to depth {to_depth}",
                    cell_name(*row, *col)
                );
            }
            SolveEvent::SolutionFound => {
                let _ = writeln!(out, "step {step}: solution found");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k16_trace_formats_deduction_steps() {
        let events = vec![
            SolveEvent::Deduced {
                row: 0,
                col: 1,
                value: Cell::One,
                strategy: StrategyId::AvoidTriple,
                reason: Reason::Gap {
                    a: (0, 0),
                    b: (0, 2),
                    value: Cell::Zero,
                },
            },
            SolveEvent::SolutionFound,
        ];
        let trace = format_trace(&events);
        assert_eq!(
            trace,
            "step 1: AvoidTriple — r0c1 = 1 (cells r0c0 and r0c2 are both 0, the cell between must differ)\nstep 2: solution found\n"
        );
    }
}
