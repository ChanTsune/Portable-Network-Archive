#![cfg(not(target_family = "wasm"))]
use crate::utils::{archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::{collections::HashSet, fs};

/// Precondition: A file exists and is referenced by a path that starts with `..` (e.g. `../in/file.txt`).
/// Action: Run `pna create` from a subdirectory and pass the `../...` path.
/// Expectation: The stored entry name is sanitized and does not contain `..`.
#[test]
fn create_command_sanitizes_parent_components_in_input_paths() {
    setup();

    fs::create_dir_all("create_sanitize_parent_components/in/").unwrap();
    fs::create_dir_all("create_sanitize_parent_components/work/").unwrap();
    fs::write("create_sanitize_parent_components/in/file.txt", b"payload").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.current_dir("create_sanitize_parent_components/work/");
    cmd.args([
        "--quiet",
        "c",
        "../archive.pna",
        "--overwrite",
        "../in/file.txt",
    ]);
    cmd.assert().success();

    let mut seen = HashSet::new();
    archive::for_each_entry("create_sanitize_parent_components/archive.pna", |entry| {
        seen.insert(entry.name().to_string());
    })
    .unwrap();

    for name in &seen {
        assert!(
            !name.split('/').any(|c| c == ".."),
            "unsanitized entry name found: {name}"
        );
    }

    assert_eq!(seen.len(), 1, "unexpected entries found: {seen:?}");
    assert!(
        seen.contains("in/file.txt"),
        "required entry missing: {seen:?}"
    );
}
