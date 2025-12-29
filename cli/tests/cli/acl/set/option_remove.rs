use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;
use predicates::prelude::*;

/// Precondition: A pre-built archive (zstd.pna) is available with user and group ACLs set.
/// Action: Remove group ACL with `-x g:test_group`.
/// Expectation: Group ACL is removed but user ACL remains.
#[test]
fn acl_set_remove() {
    setup();
    TestResources::extract_in("zstd.pna", "acl_set_remove/").unwrap();

    // Set user ACL
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "-f",
        "acl_set_remove/zstd.pna",
        "raw/text.txt",
        "-m",
        "u:test:r,w,x",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set group ACL
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "-f",
        "acl_set_remove/zstd.pna",
        "raw/text.txt",
        "-m",
        "g:test_group:r,w,x",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify both ACLs exist before removal
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_set_remove/zstd.pna",
            "raw/text.txt",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(":u:test:allow:r|w|x")
                .and(predicate::str::contains(":g:test_group:allow:r|w|x")),
        );

    // Remove group ACL with -x
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "-f",
        "acl_set_remove/zstd.pna",
        "raw/text.txt",
        "-x",
        "g:test_group",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify group ACL was removed but user ACL remains
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_set_remove/zstd.pna",
            "raw/text.txt",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(":u:test:allow:r|w|x")
                .and(predicate::str::contains(":g:test_group:allow:r|w|x").not()),
        );
}
