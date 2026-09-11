use crate::utils::{TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

fn archive_bytes() -> Vec<u8> {
    TestResources::get("zstd.pna").unwrap().data.into_owned()
}

#[test]
fn read_only_commands_accept_archive_stdin() {
    setup();
    let cases: &[(&[&str], &str)] = &[
        (&["experimental", "chunk", "list"], "AHED"),
        (&["list", "--format", "line"], "raw/text.txt"),
        (&["experimental", "verify"], "total: 9, ok: 9"),
        (
            &["experimental", "acl", "get", "raw/text.txt"],
            "# file: raw/text.txt",
        ),
        (&["xattr", "get", "raw/text.txt"], "# file: raw/text.txt"),
    ];

    for (args, expected) in cases {
        cargo_bin_cmd!("pna")
            .args(*args)
            .write_stdin(archive_bytes())
            .assert()
            .success()
            .stdout(predicate::str::contains(*expected));
    }
}

#[test]
fn diff_accepts_archive_stdin_without_changing_comparison_semantics() {
    setup();
    let dir = "stdio_read_diff";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(format!("{dir}/raw")).unwrap();
    fs::write(
        format!("{dir}/raw/text.txt"),
        TestResources::get("raw/text.txt").unwrap().data,
    )
    .unwrap();

    cargo_bin_cmd!("pna")
        .current_dir(dir)
        .args(["experimental", "diff", "raw/text.txt"])
        .write_stdin(archive_bytes())
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn extract_accepts_archive_stdin() {
    setup();
    let out = "stdio_read_extract/out";
    let _ = fs::remove_dir_all("stdio_read_extract");

    cargo_bin_cmd!("pna")
        .args(["extract", "--out-dir", out, "raw/text.txt"])
        .write_stdin(archive_bytes())
        .assert()
        .success();

    assert_eq!(
        fs::read(format!("{out}/raw/text.txt")).unwrap(),
        TestResources::get("raw/text.txt")
            .unwrap()
            .data
            .into_owned()
    );
}

#[test]
fn explicit_dash_remains_a_literal_archive_path() {
    setup();
    let dir = "stdio_read_dash";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{dir}/-"), archive_bytes()).unwrap();

    cargo_bin_cmd!("pna")
        .current_dir(dir)
        .args(["list", "--file", "-", "--format", "line"])
        .assert()
        .success()
        .stdout(predicate::str::contains("raw/text.txt"));
}
