use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use pna::Archive;
use portable_network_archive::cli;
use std::{fs, io::Cursor};

fn assert_pure_archive_stdout(args: &[&str]) -> Vec<u8> {
    let output = cargo_bin_cmd!("pna").args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output_len = output.stdout.len();
    let mut archive = Archive::read_header(Cursor::new(output.stdout)).unwrap_or_else(|error| {
        panic!("{args:?} prefixed archive stdout with non-PNA data: {error}")
    });
    for entry in archive.raw_entries() {
        entry.unwrap_or_else(|error| panic!("{args:?} emitted an invalid archive: {error}"));
    }
    let consumed = usize::try_from(archive.into_inner().position()).unwrap();
    assert_eq!(
        consumed, output_len,
        "{args:?} appended non-archive data after the PNA end marker"
    );
    output.stderr
}

#[test]
fn archive_producers_keep_verbose_diagnostics_off_stdout() {
    setup();
    fs::create_dir_all("stdout_purity").unwrap();
    fs::write("stdout_purity/input.txt", b"input").unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "create",
        "--file",
        "stdout_purity/base.pna",
        "--overwrite",
        "stdout_purity/input.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let create_stderr =
        assert_pure_archive_stdout(&["--verbose", "create", "stdout_purity/input.txt"]);
    assert!(
        String::from_utf8_lossy(&create_stderr).contains("Create an archive"),
        "the test must exercise a real verbose diagnostic"
    );

    let cases: &[&[&str]] = &[
        &[
            "--verbose",
            "xattr",
            "set",
            "--file",
            "stdout_purity/base.pna",
            "stdout_purity/input.txt",
            "--name",
            "user.stdout-purity",
            "--value",
            "value",
        ],
        &["--verbose", "sort", "--file", "stdout_purity/base.pna"],
        &[
            "--verbose",
            "experimental",
            "update",
            "--file",
            "stdout_purity/base.pna",
            "stdout_purity/input.txt",
        ],
        &[
            "--verbose",
            "append",
            "--file",
            "stdout_purity/base.pna",
            "stdout_purity/input.txt",
        ],
        &["--verbose", "concat", "--file", "stdout_purity/base.pna"],
    ];
    for args in cases {
        assert_pure_archive_stdout(args);
    }
}
