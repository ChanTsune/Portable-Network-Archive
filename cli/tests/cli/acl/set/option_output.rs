use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;
use predicates::prelude::*;
use std::fs;

/// Precondition: A pre-built archive (zstd.pna) is available.
/// Action: Set user ACL with `--output` to a new path.
/// Expectation: The output archive has the ACL; the original does not.
#[test]
fn acl_set_output() {
    setup();
    let _ = std::fs::remove_file("acl_set_output/out.pna");
    TestResources::extract_in("zstd.pna", "acl_set_output/").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "-f",
        "acl_set_output/zstd.pna",
        "--output",
        "acl_set_output/out.pna",
        "raw/text.txt",
        "-m",
        "u:test:r,w,x",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_set_output/out.pna",
            "raw/text.txt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(":u:test:allow:r|w|x"));

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_set_output/zstd.pna",
            "raw/text.txt",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(":u:")
                .not()
                .and(predicate::str::contains(":g:").not()),
        );
}

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run `pna acl set` with `--output` pointing at the existing file.
/// Expectation: The command fails without touching either file.
#[test]
fn acl_set_output_without_overwrite_refuses_to_clobber() {
    setup();
    TestResources::extract_in("zstd.pna", "acl_overwrite/").unwrap();
    fs::write("acl_overwrite/out.pna", b"sentinel").unwrap();

    let error = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "-f",
        "acl_overwrite/zstd.pna",
        "--output",
        "acl_overwrite/out.pna",
        "raw/text.txt",
        "-m",
        "u:test:r,w,x",
    ])
    .unwrap()
    .execute()
    .unwrap_err();

    assert!(format!("{error:?}").contains("already exists"));
    assert_eq!(fs::read("acl_overwrite/out.pna").unwrap(), b"sentinel");
}

/// Precondition: An archive and a pre-existing output file exist.
/// Action: Run the same set with `--overwrite`.
/// Expectation: The output holds the ACL; the original does not.
#[test]
fn acl_set_output_with_overwrite_replaces() {
    setup();
    TestResources::extract_in("zstd.pna", "acl_overwrite_ok/").unwrap();
    fs::write("acl_overwrite_ok/out.pna", b"sentinel").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "-f",
        "acl_overwrite_ok/zstd.pna",
        "--output",
        "acl_overwrite_ok/out.pna",
        "--overwrite",
        "raw/text.txt",
        "-m",
        "u:test:r,w,x",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_overwrite_ok/out.pna",
            "raw/text.txt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(":u:test:allow:r|w|x"));

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_overwrite_ok/zstd.pna",
            "raw/text.txt",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(":u:")
                .not()
                .and(predicate::str::contains(":g:").not()),
        );
}
