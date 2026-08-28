//! K10: the command-line generator. Owns everything the core may not
//! touch — files, terminal, signals, threads (AR1). The real interface
//! arrives in L5; L0 only proves the workspace builds and runs.

fn main() {
    println!(
        "binforge {} — no commands yet (L0 skeleton)",
        env!("CARGO_PKG_VERSION")
    );
}
