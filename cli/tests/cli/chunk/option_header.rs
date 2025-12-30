//! Tests for the --header option which adds column headers to output.

use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

/// Precondition: A valid PNA archive exists.
/// Action: Run `pna experimental chunk list --header -f <archive>`.
/// Expectation: Output includes header row with column names.
#[test]
fn chunk_list_with_header() {
    setup();
    TestResources::extract_in("zstd.pna", "chunk_list_header/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--header",
            "-f",
            "chunk_list_header/zstd.pna",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Index")
                .and(predicate::str::contains("Type"))
                .and(predicate::str::contains("Size"))
                .and(predicate::str::contains("Offset")),
        );
}

/// Precondition: A valid PNA archive exists.
/// Action: Run `pna experimental chunk list -h -f <archive>` (short form).
/// Expectation: Output includes header row with column names.
#[test]
fn chunk_list_with_header_short() {
    setup();
    TestResources::extract_in("store.pna", "chunk_list_header_short/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "-h",
            "-f",
            "chunk_list_header_short/store.pna",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Index"));
}

/// Precondition: A valid PNA archive exists.
/// Action: Run `pna experimental chunk list --header --long -f <archive>`.
/// Expectation: Output includes Body column in header.
#[test]
fn chunk_list_header_with_long() {
    setup();
    TestResources::extract_in("store.pna", "chunk_list_header_long/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--header",
            "--long",
            "-f",
            "chunk_list_header_long/store.pna",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Index")
                .and(predicate::str::contains("Type"))
                .and(predicate::str::contains("Size"))
                .and(predicate::str::contains("Offset"))
                .and(predicate::str::contains("Body")),
        );
}
