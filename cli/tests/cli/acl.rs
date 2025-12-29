#[cfg(not(target_family = "wasm"))]
mod dump;
mod missing_file;
#[cfg(not(target_family = "wasm"))]
mod restore;

use crate::utils::{EmbedExt, TestResources, diff::diff, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;
use predicates::prelude::*;

/// Precondition: Raw test resources are available.
/// Action: Create archive, set user ACL, set group ACL, remove group ACL, get ACL, extract.
/// Expectation: Each ACL operation is verified to have the expected effect.
#[test]
fn archive_acl_get_set() {
    setup();
    TestResources::extract_in("raw/", "acl_get_set/in/").unwrap();

    // Create archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "acl_get_set/acl_get_set.pna",
        "--overwrite",
        "acl_get_set/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Set user ACL with -m (modify)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "acl",
        "set",
        "-f",
        "acl_get_set/acl_get_set.pna",
        "acl_get_set/in/raw/text.txt",
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
            "acl_get_set/acl_get_set.pna",
            "acl_get_set/in/raw/text.txt",
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
        "acl_get_set/acl_get_set.pna",
        "acl_get_set/in/raw/text.txt",
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
            "acl_get_set/acl_get_set.pna",
            "acl_get_set/in/raw/text.txt",
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
        "acl_get_set/acl_get_set.pna",
        "acl_get_set/in/raw/text.txt",
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
            "acl_get_set/acl_get_set.pna",
            "acl_get_set/in/raw/text.txt",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(":u:test:allow:r|w|x")
                .and(predicate::str::contains(":g:test_group:allow:r|w|x").not()),
        );

    // Verify non-target entries have no ACLs (using wildcard to get all entries)
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            "acl_get_set/acl_get_set.pna",
            "acl_get_set/in/raw/empty.txt",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(":u:")
                .not()
                .and(predicate::str::contains(":g:").not()),
        );

    // Extract archive
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "acl_get_set/acl_get_set.pna",
        "--overwrite",
        "--out-dir",
        "acl_get_set/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    diff("acl_get_set/in/", "acl_get_set/out/").unwrap();
}
