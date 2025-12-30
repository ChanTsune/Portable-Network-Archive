//! Tests for the --header option which adds column headers to output.

use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An empty PNA archive exists.
/// Action: Run `pna experimental chunk list --header -f <archive>`.
/// Expectation: Output includes header row with Index, Type, Size, Offset columns.
#[test]
fn chunk_list_with_header() {
    setup();
    TestResources::extract_in("empty.pna", "chunk_list_header/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--header",
            "-f",
            "chunk_list_header/empty.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            " Index  Type  Size  Offset \n",
            " 1      AHED  8     0x0008 \n",
            " 2      AEND  0     0x001c \n",
        ));
}

/// Precondition: An empty PNA archive exists.
/// Action: Run `pna experimental chunk list -h -f <archive>` (short form).
/// Expectation: Output includes header row with column names.
#[test]
fn chunk_list_with_header_short() {
    setup();
    TestResources::extract_in("empty.pna", "chunk_list_header_short/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "-h",
            "-f",
            "chunk_list_header_short/empty.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            " Index  Type  Size  Offset \n",
            " 1      AHED  8     0x0008 \n",
            " 2      AEND  0     0x001c \n",
        ));
}

/// Precondition: An empty PNA archive exists.
/// Action: Run `pna experimental chunk list --header --long -f <archive>`.
/// Expectation: Output includes Body column in header.
///
/// Note: With --long, the Body column may contain binary data (null bytes),
/// so this test uses byte-level assertions rather than exact string match.
#[test]
fn chunk_list_header_with_long() {
    setup();
    TestResources::extract_in("empty.pna", "chunk_list_header_long/").unwrap();

    let output = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--header",
            "--long",
            "-f",
            "chunk_list_header_long/empty.pna",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);

    // Verify header row contains "Body" column
    assert!(
        output_str.contains("Index") && output_str.contains("Body"),
        "Header should contain Index and Body columns"
    );
    // Verify chunk types are present
    assert!(
        output.windows(4).any(|w| w == b"AHED"),
        "Output should contain AHED chunk"
    );
    assert!(
        output.windows(4).any(|w| w == b"AEND"),
        "Output should contain AEND chunk"
    );
}
