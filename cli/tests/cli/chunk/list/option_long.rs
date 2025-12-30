//! Tests for the --long option which displays chunk body content.
//!
//! Note: The Body column may contain binary data (including null bytes),
//! so these tests use byte-level assertions rather than string exact matches.

use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An empty PNA archive exists.
/// Action: Run `pna experimental chunk list --long -f <archive>`.
/// Expectation: Output includes Body column (wider format than without --long).
#[test]
fn chunk_list_long_shows_body() {
    setup();
    TestResources::extract_in("empty.pna", "chunk_list_long/").unwrap();

    let output = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--long",
            "-f",
            "chunk_list_long/empty.pna",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // With --long, AHED chunk body contains 8 null bytes (archive header version)
    // The output should contain the chunk types and have body data
    assert!(output.len() > 50, "Output should be longer with body data");
    // Verify AHED and AEND chunks are present
    assert!(
        output.windows(4).any(|w| w == b"AHED"),
        "Output should contain AHED chunk"
    );
    assert!(
        output.windows(4).any(|w| w == b"AEND"),
        "Output should contain AEND chunk"
    );
}

/// Precondition: A deflate-compressed archive exists.
/// Action: Run `pna experimental chunk list --long -f <archive>`.
/// Expectation: Output contains hex offsets (0x prefix) in Offset column.
#[test]
fn chunk_list_long_shows_hex_offsets() {
    setup();
    TestResources::extract_in("deflate.pna", "chunk_list_long_hex/").unwrap();

    let output = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--long",
            "-f",
            "chunk_list_long_hex/deflate.pna",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Verify output contains hex offset format (0x prefix)
    let output_str = String::from_utf8_lossy(&output);
    assert!(
        output_str.contains("0x0008"),
        "Output should contain hex offset 0x0008"
    );
    // Verify file entry chunks are present
    assert!(
        output.windows(4).any(|w| w == b"FHED"),
        "Output should contain FHED chunk"
    );
    assert!(
        output.windows(4).any(|w| w == b"FDAT"),
        "Output should contain FDAT chunk"
    );
}

/// Precondition: An empty PNA archive exists.
/// Action: Run `pna experimental chunk list -l -f <archive>` (short form).
/// Expectation: Output includes body content (same as --long).
#[test]
fn chunk_list_long_short_form() {
    setup();
    TestResources::extract_in("empty.pna", "chunk_list_long_short/").unwrap();

    let output = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "-l",
            "-f",
            "chunk_list_long_short/empty.pna",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Short form -l should produce same output as --long
    assert!(output.len() > 50, "Output should be longer with body data");
    assert!(
        output.windows(4).any(|w| w == b"AHED"),
        "Output should contain AHED chunk"
    );
    assert!(
        output.windows(4).any(|w| w == b"AEND"),
        "Output should contain AEND chunk"
    );
}
