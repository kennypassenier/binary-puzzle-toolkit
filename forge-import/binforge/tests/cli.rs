//! M6/K10 end-to-end: the real binary, real files, real exit codes
//! (standing rule 9 — nothing mocked that can be real).

use std::process::Command;

fn binforge() -> Command {
    Command::new(env!("CARGO_BIN_EXE_binforge"))
}

#[test]
fn m6_inspect_renders_a_builtin_type() {
    let out = binforge().args(["inspect", "4x6x6"]).output().unwrap();
    assert!(out.status.success(), "exit: {:?}", out.status);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("4x6x6 — 12x12, 5 region(s)"), "{text}");
    assert!(text.contains("every cell constrained"), "{text}");
}

#[test]
fn m6_inspect_accepts_a_bare_standard_size() {
    let out = binforge().args(["inspect", "10"]).output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("(standard, untagged) — 10x10"), "{text}");
}

#[test]
fn m6_inspect_reads_an_invented_type_from_a_real_file() {
    // The K9 workflow in miniature: write a geometry by hand, look at it
    // before anything tries to generate from it.
    let dir = std::env::temp_dir().join(format!("binforge-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("4x10x10.toml");
    std::fs::write(
        &path,
        "tag = \"4x10x10\"\nsize = 20\n\
         [[regions]]\nrow = 0\ncol = 0\nrows = 20\ncols = 20\n\
         [[regions]]\nrow = 0\ncol = 0\nrows = 10\ncols = 10\n\
         [[regions]]\nrow = 0\ncol = 10\nrows = 10\ncols = 10\n\
         [[regions]]\nrow = 10\ncol = 0\nrows = 10\ncols = 10\n\
         [[regions]]\nrow = 10\ncol = 10\nrows = 10\ncols = 10\n",
    )
    .unwrap();

    let out = binforge()
        .args(["inspect", "--file"])
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("4x10x10 — 20x20, 5 region(s)"), "{text}");
    assert!(text.contains("every cell constrained"), "{text}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn k10_a_broken_geometry_file_exits_two_and_explains_itself() {
    let dir = std::env::temp_dir().join(format!("binforge-cli-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("broken.toml");
    // Second region hangs off the right edge: the off-by-one that a
    // hand-written geometry actually produces.
    std::fs::write(
        &path,
        "size = 12\n\
         [[regions]]\nrow = 0\ncol = 0\nrows = 12\ncols = 12\n\
         [[regions]]\nrow = 0\ncol = 8\nrows = 6\ncols = 6\n",
    )
    .unwrap();

    let out = binforge()
        .args(["inspect", "--file"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "usage/geometry errors exit 2 (AR11)"
    );
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("region 1"),
        "must name the offending region: {err}"
    );
    assert!(err.contains("Remedy:"), "must carry a remedy: {err}");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn k10_an_unknown_tag_lists_what_is_available() {
    let out = binforge().args(["inspect", "9x9x9"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("unknown geometry"), "{err}");
    assert!(
        err.contains("4x6x6"),
        "the remedy lists the built-ins: {err}"
    );
}

#[test]
fn k10_types_lists_every_builtin() {
    let out = binforge().arg("types").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    for tag in binforge_core::geometry::BUILTIN_TAGS {
        assert!(text.contains(tag), "{tag} missing from: {text}");
    }
}
