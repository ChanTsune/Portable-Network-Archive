//! Tests for basic chunk list functionality.

use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

/// Precondition: A valid PNA archive exists.
/// Action: Run `pna experimental chunk list -f <archive>`.
/// Expectation: Chunks are listed with Index, Type, Size, and Offset columns.
#[test]
fn chunk_list_basic() {
    setup();
    TestResources::extract_in("zstd.pna", "chunk_list_basic/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "-f",
            "chunk_list_basic/zstd.pna",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("AHED")
                .and(predicate::str::contains("FHED"))
                .and(predicate::str::contains("FDAT"))
                .and(predicate::str::contains("FEND"))
                .and(predicate::str::contains("AEND")),
        );
}

/// Precondition: A valid PNA archive exists.
/// Action: Run `pna experimental chunk list -f <archive>`.
/// Expectation: Output contains hex offset values in 0x format.
#[test]
fn chunk_list_shows_hex_offsets() {
    setup();
    TestResources::extract_in("zstd.pna", "chunk_list_hex/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "-f",
            "chunk_list_hex/zstd.pna",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("0x"));
}

/// Precondition: A valid PNA archive exists.
/// Action: Run `pna experimental chunk list -f <archive>`.
/// Expectation: First chunk is AHED (archive header), last chunk is AEND (archive end).
#[test]
fn chunk_list_archive_structure() {
    setup();
    TestResources::extract_in("store.pna", "chunk_list_structure/").unwrap();

    let output = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "-f",
            "chunk_list_structure/store.pna",
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
