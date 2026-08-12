//! binsolve CLI — thin frontend over binsolve-core. Real interface
//! lands in L6 (clap, batch, --explain, --check); until then this is
//! the L0 walking skeleton.

#![forbid(unsafe_code)]

fn main() {
    // Exit code 2 = usage error (AR7): no interface exists yet.
    eprintln!("binsolve: not yet implemented — milestone L6 delivers the CLI");
    std::process::exit(2);
}
