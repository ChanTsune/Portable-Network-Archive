use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;
use predicates::prelude::*;

/// Precondition: A pre-built archive (zstd.pna) is available.
/// Action: Set user ACL with `--output` to a new path.
/// Expectation: The output archive has the ACL; the original does not.
#[test]
fn acl_set_output() {
    setup();
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
