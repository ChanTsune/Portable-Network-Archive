//! Tests for chunk listing of solid archives.

use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

/// Precondition: A solid mode archive exists.
/// Action: Run `pna experimental chunk list -f <solid_archive>`.
/// Expectation: Output contains solid-specific chunk types (SHED, SDAT).
#[test]
fn chunk_list_solid_archive() {
    setup();
    TestResources::extract_in("solid_store.pna", "chunk_list_solid/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "-f",
            "chunk_list_solid/solid_store.pna",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("SHED")
                .and(predicate::str::contains("SDAT"))
                .and(predicate::str::contains("AHED"))
                .and(predicate::str::contains("AEND")),
        );
}

/// Precondition: A solid mode archive exists.
/// Action: Run `pna experimental chunk list --header -f <solid_archive>`.
/// Expectation: Solid chunk types are listed with proper headers.
#[test]
fn chunk_list_solid_with_header() {
    setup();
    TestResources::extract_in("solid_zstd.pna", "chunk_list_solid_header/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--header",
            "-f",
            "chunk_list_solid_header/solid_zstd.pna",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Index")
                .and(predicate::str::contains("SHED"))
                .and(predicate::str::contains("SDAT")),
        );
}

/// Precondition: A solid mode archive with compression exists.
/// Action: Run `pna experimental chunk list -f <solid_archive>`.
/// Expectation: Archive structure starts with AHED and ends with AEND.
#[test]
fn chunk_list_solid_structure() {
    setup();
    TestResources::extract_in("solid_zstd.pna", "chunk_list_solid_struct/").unwrap();

    let output = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "-f",
            "chunk_list_solid_struct/solid_zstd.pna",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    let lines: Vec<&str> = output_str.lines().collect();

    // First chunk should be AHED
    assert!(
        lines.first().is_some_and(|l| l.contains("AHED")),
        "First chunk should be AHED"
    );

    // Last chunk should be AEND
    assert!(
        lines.last().is_some_and(|l| l.contains("AEND")),
        "Last chunk should be AEND"
    );
}
