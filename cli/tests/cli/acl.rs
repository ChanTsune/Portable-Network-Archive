#[cfg(not(target_family = "wasm"))]
mod dump;
mod missing_file;
#[cfg(not(target_family = "wasm"))]
mod restore;

use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;
use predicates::prelude::*;

/// Precondition: A pre-built archive (zstd.pna) is available.
/// Action: Set user ACL, set group ACL, remove group ACL, get ACL.
/// Expectation: Each ACL operation is verified to have the expected effect.
#[test]
fn archive_acl_get_set() {
    setup();
    TestResources::extract_in("zstd.pna", "acl_get_set/").unwrap();

    // Set user ACL with -m (modify)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "-f",
        "acl_get_set/zstd.pna",
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
            "acl_get_set/zstd.pna",
            "raw/text.txt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(":u:test:allow:r|w|x"));

    // Set group ACL with -m (modify)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "-f",
        "acl_get_set/zstd.pna",
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
            "acl_get_set/zstd.pna",
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
        "acl_get_set/zstd.pna",
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
            "acl_get_set/zstd.pna",
            "raw/text.txt",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(":u:test:allow:r|w|x")
                .and(predicate::str::contains(":g:test_group:allow:r|w|x").not()),
        );

    // Verify non-target entries have no ACLs
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_get_set/zstd.pna",
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
