//! Tests for the --long option which displays chunk body content.

use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

/// Precondition: A valid PNA archive with text file entries exists.
/// Action: Run `pna experimental chunk list --long -f <archive>`.
/// Expectation: Output includes chunk body content for text data.
#[test]
fn chunk_list_long_shows_body() {
    setup();
    TestResources::extract_in("store.pna", "chunk_list_long/").unwrap();

    let output = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--long",
            "-f",
            "chunk_list_long/store.pna",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);

    // With --long, output should have more columns (Body column)
    // The output line length should be greater than without --long
    assert!(
        output_str.lines().any(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            // Without --long: [Index, Type, Size, Offset]
            // With --long: [Index, Type, Size, Offset, Body...]
            parts.len() > 4
        }),
        "Output should include body content"
    );
}

/// Precondition: A valid PNA archive with binary data exists.
/// Action: Run `pna experimental chunk list --long -f <archive>`.
/// Expectation: Binary data is displayed in hex format (0x prefix).
#[test]
fn chunk_list_long_binary_as_hex() {
    setup();
    TestResources::extract_in("zstd.pna", "chunk_list_long_binary/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--long",
            "-f",
            "chunk_list_long_binary/zstd.pna",
        ])
        .assert()
        .success()
        // Binary data should be shown with 0x prefix (hex format)
        .stdout(predicate::str::contains("0x"));
}

/// Precondition: A valid PNA archive exists.
/// Action: Run `pna experimental chunk list -l -f <archive>` (short form).
/// Expectation: Output includes body content (same as --long).
#[test]
fn chunk_list_long_short_form() {
    setup();
    TestResources::extract_in("store.pna", "chunk_list_long_short/").unwrap();

    let output = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "-l",
            "-f",
            "chunk_list_long_short/store.pna",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);

    // Body content should be present
    assert!(
        output_str.lines().any(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.len() > 4
        }),
        "Short form -l should also include body content"
    );
}
