//! M4 meta-test: every corpus file parses, and every recorded solution
//! is valid under the puzzle's full region decomposition — which also
//! continuously re-verifies the AR3 all-rules assumption on real data.

use binsolve_core::parse::parse_corpus_file;
use binsolve_core::region::{validate_givens, validate_solution};
use std::fs;
use std::path::PathBuf;

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../corpus")
}

fn collect_txt(dir: &PathBuf, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("corpus directory readable") {
        let path = entry.expect("corpus entry readable").path();
        if path.is_dir() {
            collect_txt(&path, out);
        } else if path.extension().is_some_and(|e| e == "txt") {
            out.push(path);
        }
    }
}

#[test]
fn m4_corpus_parses_and_solutions_validate() {
    let mut files = Vec::new();
    collect_txt(&corpus_root(), &mut files);
    assert!(
        files.len() >= 8,
        "seed corpus expected at least 8 files, found {}",
        files.len()
    );
    for path in files {
        let content =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let (puzzle, solution) =
            parse_corpus_file(&content).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        if let Some(solution) = solution {
            let given_violations = validate_givens(&puzzle.givens, &solution);
            assert!(
                given_violations.is_empty(),
                "{}: {}",
                path.display(),
                given_violations[0]
            );
            let violations = validate_solution(&solution, &puzzle.regions());
            assert!(
                violations.is_empty(),
                "{}: solution violates rules: {}",
                path.display(),
                violations[0]
            );
        }
    }
}
