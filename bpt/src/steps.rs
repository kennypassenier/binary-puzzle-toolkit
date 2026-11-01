//! `bpt steps`: the by-hand solution of one puzzle, one numbered line per
//! step with the technique and the reason (feat-human-1). The proof that
//! a puzzle is solvable by a person is that this list reaches the end.

use bpt_core::grid::Cell;
use bpt_core::hand::{HandResult, Step, Why};
use bpt_core::strategy::Reason;

/// A cell as people name it: row and column counted from 1.
fn at((row, col): (usize, usize)) -> String {
    format!("r{}c{}", row + 1, col + 1)
}

fn digit(cell: Cell) -> char {
    cell.to_char()
}

fn line_name(is_row: bool, index: usize) -> String {
    format!("{} {}", if is_row { "row" } else { "column" }, index + 1)
}

pub fn reason(step: &Step) -> String {
    match step.why {
        Why::Rule(Reason::AdjacentPair { a, b, value }) => format!(
            "{} and {} are both {}: a third {} would make three in a row",
            at(a),
            at(b),
            digit(value),
            digit(value)
        ),
        Why::Rule(Reason::Gap { a, b, value }) => format!(
            "{} and {} are both {} with one cell between: that cell cannot be {} too",
            at(a),
            at(b),
            digit(value),
            digit(value)
        ),
        Why::Rule(Reason::CountComplete {
            is_row,
            line,
            value,
            need,
        }) => format!(
            "{} already holds all {need} of its {}s: the rest are {}s",
            line_name(is_row, line),
            digit(value),
            digit(value.opposite().expect("filled")),
        ),
        Why::Rule(Reason::UniqueSwap { is_row, other }) => format!(
            "{} is finished, and this line would copy it unless these two cells differ from it",
            line_name(is_row, other)
        ),
        Why::Rule(_) => "forced by the rules".to_owned(),
        Why::Capacity {
            tried,
            digit: d,
            needed,
            at_most: Some(m),
        } => format!(
            "a {} here leaves room for at most {m} {}s in this line, which needs {needed}",
            digit(tried),
            digit(d)
        ),
        Why::Capacity { tried, .. } => format!(
            "a {} here leaves no way to finish this line without three in a row",
            digit(tried)
        ),
    }
}

pub fn render(result: &HandResult) -> String {
    let mut out = String::new();
    for (n, step) in result.steps.iter().enumerate() {
        out.push_str(&format!(
            "{:>4}. {} = {}  [{}]  {}\n",
            n + 1,
            at((step.row, step.col)),
            digit(step.value),
            step.technique.name(),
            reason(step)
        ));
    }
    out.push_str(&if result.solved {
        format!("solved by hand in {} steps\n", result.steps.len())
    } else {
        format!(
            "NOT solvable by hand: stuck after {} steps\n",
            result.steps.len()
        )
    });
    out
}
