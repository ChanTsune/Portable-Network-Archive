use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;
use predicates::prelude::*;

/// Precondition: A pre-built archive (zstd.pna) is available.
/// Action: Set user ACL with `-m u:test:r,w,x`.
/// Expectation: User ACL is applied and verified via `acl get`.
#[test]
fn acl_set_modify_user() {
    setup();
    TestResources::extract_in("zstd.pna", "acl_set_modify_user/").unwrap();

    // Set user ACL with -m (modify)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "-f",
        "acl_set_modify_user/zstd.pna",
        "raw/text.txt",
        "-m",
        "u:test:r,w,x",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify user ACL was applied
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_set_modify_user/zstd.pna",
            "raw/text.txt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(":u:test:allow:r|w|x"));

    // Verify non-target entries have no ACLs
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_set_modify_user/zstd.pna",
            "raw/empty.txt",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(":u:")
                .not()
                .and(predicate::str::contains(":g:").not()),
        );
}

/// Precondition: A pre-built archive (zstd.pna) is available.
/// Action: Set user ACL, then set group ACL with `-m g:test_group:r,w,x`.
/// Expectation: Both user and group ACLs exist on the entry.
#[test]
fn acl_set_modify_group() {
    setup();
    TestResources::extract_in("zstd.pna", "acl_set_modify_group/").unwrap();

    // Set user ACL first
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "-f",
        "acl_set_modify_group/zstd.pna",
        "raw/text.txt",
        "-m",
        "u:test:r,w,x",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set group ACL with -m (modify)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "-f",
        "acl_set_modify_group/zstd.pna",
        "raw/text.txt",
        "-m",
        "g:test_group:r,w,x",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify both user and group ACLs exist
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_set_modify_group/zstd.pna",
            "raw/text.txt",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(":u:test:allow:r|w|x")
                .and(predicate::str::contains(":g:test_group:allow:r|w|x")),
        );
}
