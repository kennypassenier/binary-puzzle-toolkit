//! feat-report-1: how hard is a collection of puzzles, and can a person
//! solve every one of them? Pure logic — the CLI reads the files and
//! hands over located lines (AR1).
//!
//! "Solvable by a person" is taken from binarypuzzle.com's own corpus,
//! every puzzle of which is measured to have one solution reachable
//! without guessing: levels L1 to L3. Anything else is listed by
//! location, because a website serving such a puzzle is the fault this
//! report exists to catch.

use crate::grade::{Level, level_for_tier};
use bpt_core::event::{EventLog, NullObserver, SolveEvent};
use bpt_core::region::Puzzle;
use bpt_core::search::{SolveMode, SolveOutcome, solve, solve_within};
use bpt_core::strategy::StrategyId;

/// Every strategy in ladder order, which is also the order the report
/// prints them in.
pub const STRATEGIES: [StrategyId; 6] = [
    StrategyId::FindDuo,
    StrategyId::AvoidTriple,
    StrategyId::FillByCount,
    StrategyId::KeepLineUnique,
    StrategyId::CountingArgument,
    StrategyId::FillPossibilities,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Solved by reasoning alone. Every deduction holds in every
    /// solution, so a grid finished this way is also the only solution.
    Level(Level),
    /// One solution, but reasoning stalls before it: someone has to guess.
    NeedsGuessing,
    NotUnique,
    NoSolution,
    /// The uniqueness search ran out of its node budget (B4). A claim
    /// about the search, not the puzzle, so it is never counted as either.
    Undecided,
    /// The line did not parse; the CLI knows why and says so.
    Invalid,
}

impl Verdict {
    /// One solution, no guessing: what a person can finish with logic.
    pub fn human_solvable(&self) -> bool {
        matches!(self, Verdict::Level(_))
    }

    pub fn label(&self) -> &'static str {
        match self {
            Verdict::Level(Level::L1) => "L1 patterns and counts",
            Verdict::Level(Level::L2) => "L2 cross-line reasoning",
            Verdict::Level(Level::L3) => "L3 line enumeration",
            Verdict::Level(Level::L4) | Verdict::NeedsGuessing => "L4 needs guessing",
            Verdict::NotUnique => "not unique",
            Verdict::NoSolution => "no solution",
            Verdict::Undecided => "undecided (budget)",
            Verdict::Invalid => "invalid line",
        }
    }

    /// Every row, in the order the report prints them, so an empty row
    /// still shows and two reports line up.
    pub const ALL: [Verdict; 8] = [
        Verdict::Level(Level::L1),
        Verdict::Level(Level::L2),
        Verdict::Level(Level::L3),
        Verdict::NeedsGuessing,
        Verdict::NotUnique,
        Verdict::NoSolution,
        Verdict::Undecided,
        Verdict::Invalid,
    ];
}

/// One assessed puzzle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Where it came from, as the CLI names it (`path:line`).
    pub location: String,
    pub size: usize,
    pub clues: usize,
    pub verdict: Verdict,
    /// Deductions per strategy, indexed like [`STRATEGIES`]. Counted only
    /// for puzzles reasoning finishes; a stalled run says nothing about
    /// what the puzzle as a whole needs.
    pub steps: [usize; 6],
}

impl Entry {
    /// A line that did not parse still occupies a row in the report.
    pub fn invalid(location: String) -> Self {
        Entry {
            location,
            size: 0,
            clues: 0,
            verdict: Verdict::Invalid,
            steps: [0; 6],
        }
    }

    /// Clues as a share of the grid, so 6x6 and 14x14 can be compared.
    fn clue_percent(&self) -> f64 {
        if self.size == 0 {
            return 0.0;
        }
        100.0 * self.clues as f64 / (self.size * self.size) as f64
    }
}

/// Assess one puzzle. Reasoning first, which is cheap and answers most;
/// only a puzzle it cannot finish pays for the budgeted uniqueness search.
pub fn assess(location: String, puzzle: &Puzzle, budget: u64) -> Entry {
    let size = puzzle.givens.size();
    let clues = puzzle.givens.filled_count();
    let mut log = EventLog::default();
    let verdict = match solve(puzzle, SolveMode::StrategiesOnly, &mut log) {
        SolveOutcome::Solved { stats, .. } => Verdict::Level(level_for_tier(stats.max_tier)),
        SolveOutcome::Contradiction { .. } => Verdict::NoSolution,
        _ => match solve_within(
            puzzle,
            SolveMode::ProveUniqueness,
            &mut NullObserver,
            budget,
        ) {
            SolveOutcome::Solved { .. } => Verdict::NeedsGuessing,
            SolveOutcome::MultipleSolutions { .. } => Verdict::NotUnique,
            SolveOutcome::Contradiction { .. } => Verdict::NoSolution,
            SolveOutcome::BudgetExhausted { .. } | SolveOutcome::Stuck { .. } => Verdict::Undecided,
        },
    };
    let mut steps = [0; 6];
    if verdict.human_solvable() {
        for event in &log.events {
            if let SolveEvent::Deduced { strategy, .. } = event
                && let Some(i) = STRATEGIES.iter().position(|s| s == strategy)
            {
                steps[i] += 1;
            }
        }
    }
    Entry {
        location,
        size,
        clues,
        verdict,
        steps,
    }
}

fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
}

/// The report as text: one row per verdict, the human-solvable total,
/// then every puzzle outside it by location — all of them, never a sample.
pub fn render(entries: &[Entry]) -> String {
    let total = entries.len();
    let mut out = String::new();
    out.push_str(&format!(
        "{:<24} {:>7} {:>6}  {:<22} strategy steps\n",
        "verdict", "puzzles", "share", "clues % median (range)"
    ));
    for verdict in Verdict::ALL {
        let rows: Vec<&Entry> = entries.iter().filter(|e| e.verdict == verdict).collect();
        let share = if total == 0 {
            0.0
        } else {
            100.0 * rows.len() as f64 / total as f64
        };
        let clues = if rows.is_empty() || verdict == Verdict::Invalid {
            "-".to_string()
        } else {
            let mut pct: Vec<f64> = rows.iter().map(|e| e.clue_percent()).collect();
            pct.sort_by(f64::total_cmp);
            format!(
                "{:.0}% ({:.0}-{:.0}%)",
                median(&pct),
                pct[0],
                pct[pct.len() - 1]
            )
        };
        let mut steps = [0usize; 6];
        for e in &rows {
            for (total, n) in steps.iter_mut().zip(e.steps) {
                *total += n;
            }
        }
        let used: Vec<String> = STRATEGIES
            .iter()
            .zip(steps)
            .filter(|(_, n)| *n > 0)
            .map(|(s, n)| format!("{} {n}", s.name()))
            .collect();
        let used = if used.is_empty() {
            "-".to_string()
        } else {
            used.join(", ")
        };
        out.push_str(&format!(
            "{:<24} {:>7} {:>5.0}%  {:<22} {used}\n",
            verdict.label(),
            rows.len(),
            share,
            clues
        ));
    }

    let human = entries
        .iter()
        .filter(|e| e.verdict.human_solvable())
        .count();
    out.push_str(&format!(
        "\nsolvable by a person (one solution, no guessing): {human} of {total}\n"
    ));

    let outside: Vec<&Entry> = entries
        .iter()
        .filter(|e| !e.verdict.human_solvable())
        .collect();
    if !outside.is_empty() {
        out.push_str("\nnot solvable by a person:\n");
        for e in outside {
            if e.verdict == Verdict::Invalid {
                out.push_str(&format!("  {}  {}\n", e.location, e.verdict.label()));
            } else {
                out.push_str(&format!(
                    "  {}  {}  ({}x{}, {} clues)\n",
                    e.location,
                    e.verdict.label(),
                    e.size,
                    e.size,
                    e.clues
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use bpt_core::parse::parse_line;

    const BUDGET: u64 = crate::carve::UNIQUENESS_BUDGET;

    fn verdict_of(line: &str) -> Verdict {
        assess("t".into(), &parse_line(line).unwrap(), BUDGET).verdict
    }

    #[test]
    fn feat_report_1_a_published_easy_puzzle_is_solvable_by_a_person() {
        let v = verdict_of("1..0....00.1.00..1......00.1...1..00");
        assert_eq!(v, Verdict::Level(Level::L1));
        assert!(v.human_solvable());
    }

    #[test]
    fn feat_report_1_an_empty_grid_is_not_unique() {
        assert_eq!(verdict_of(&".".repeat(36)), Verdict::NotUnique);
    }

    #[test]
    fn feat_report_1_a_contradiction_has_no_solution() {
        assert_eq!(
            verdict_of(&format!("000{}", ".".repeat(33))),
            Verdict::NoSolution
        );
    }

    #[test]
    fn feat_report_1_steps_are_counted_only_when_reasoning_finishes() {
        let solved = assess(
            "a".into(),
            &parse_line("1..0....00.1.00..1......00.1...1..00").unwrap(),
            BUDGET,
        );
        assert!(solved.steps.iter().sum::<usize>() > 0);
        let empty = assess("b".into(), &parse_line(&".".repeat(36)).unwrap(), BUDGET);
        assert_eq!(empty.steps, [0; 6]);
    }

    #[test]
    fn feat_report_1_every_puzzle_outside_is_listed_by_location() {
        let entries = vec![
            assess(
                "good.txt:1".into(),
                &parse_line("1..0....00.1.00..1......00.1...1..00").unwrap(),
                BUDGET,
            ),
            assess(
                "bad.txt:4".into(),
                &parse_line(&".".repeat(36)).unwrap(),
                BUDGET,
            ),
            Entry::invalid("bad.txt:5".into()),
        ];
        let text = render(&entries);
        assert!(text.contains("solvable by a person (one solution, no guessing): 1 of 3"));
        assert!(text.contains("  bad.txt:4  not unique  (6x6, 0 clues)"));
        assert!(text.contains("  bad.txt:5  invalid line"));
        assert!(!text.contains("good.txt:1"), "a good puzzle is not listed");
    }
}
