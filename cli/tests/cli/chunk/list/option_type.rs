//! Tests for the --type and --exclude-type options.

use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;

/// Precondition: A deflate-compressed PNA archive exists.
/// Action: Run `pna experimental chunk list --type FHED -f <archive>`.
/// Expectation: Only FHED chunks are listed.
#[test]
fn chunk_list_with_type() {
    setup();
    TestResources::extract_in("deflate.pna", "chunk_list_type_fhed/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--type",
            "FHED",
            "-f",
            "chunk_list_type_fhed/deflate.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            " 2   FHED  19  0x001c  \n",
            " 5   FHED  25  0x005b  \n",
            " 8   FHED  25  0x8cce  \n",
            " 11  FHED  25  0x9018  \n",
            " 14  FHED  36  0xfe0a  \n",
            " 17  FHED  23  0xfe5d  \n",
            " 20  FHED  22  0xfec0  \n",
            " 23  FHED  26  0x1d291 \n",
            " 26  FHED  18  0x1d2d7 \n",
        ));
}

/// Precondition: A deflate-compressed PNA archive exists.
/// Action: Run `pna experimental chunk list --exclude-type FHED -f <archive>`.
/// Expectation: FHED chunks are omitted while other chunk types remain.
#[test]
fn chunk_list_with_exclude_type() {
    setup();
    TestResources::extract_in("deflate.pna", "chunk_list_exclude_fhed/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--exclude-type",
            "FHED",
            "-f",
            "chunk_list_exclude_fhed/deflate.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            " 1   AHED  8      0x0008  \n",
            " 3   FDAT  8      0x003b  \n",
            " 4   FEND  0      0x004f  \n",
            " 6   FDAT  35894  0x0080  \n",
            " 7   FEND  0      0x8cc2  \n",
            " 9   FDAT  781    0x8cf3  \n",
            " 10  FEND  0      0x900c  \n",
            " 12  FDAT  28085  0x903d  \n",
            " 13  FEND  0      0xfdfe  \n",
            " 15  FDAT  11     0xfe3a  \n",
            " 16  FEND  0      0xfe51  \n",
            " 18  FDAT  40     0xfe80  \n",
            " 19  FEND  0      0xfeb4  \n",
            " 21  FDAT  54167  0xfee2  \n",
            " 22  FEND  0      0x1d285 \n",
            " 24  FDAT  8      0x1d2b7 \n",
            " 25  FEND  0      0x1d2cb \n",
            " 27  FDAT  18     0x1d2f5 \n",
            " 28  FEND  0      0x1d313 \n",
            " 29  AEND  0      0x1d31f \n",
        ));
}

/// Precondition: A deflate-compressed PNA archive exists.
/// Action: Run `pna experimental chunk list --type FHED --type FDAT -f <archive>`.
/// Expectation: Both FHED and FDAT chunks are listed (OR semantics).
#[test]
fn chunk_list_with_multiple_types() {
    setup();
    TestResources::extract_in("deflate.pna", "chunk_list_type_multi/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--type",
            "FHED",
            "--type",
            "FDAT",
            "-f",
            "chunk_list_type_multi/deflate.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(
            " 2   FHED  19     0x001c  \n",
            " 3   FDAT  8      0x003b  \n",
            " 5   FHED  25     0x005b  \n",
            " 6   FDAT  35894  0x0080  \n",
            " 8   FHED  25     0x8cce  \n",
            " 9   FDAT  781    0x8cf3  \n",
            " 11  FHED  25     0x9018  \n",
            " 12  FDAT  28085  0x903d  \n",
            " 14  FHED  36     0xfe0a  \n",
            " 15  FDAT  11     0xfe3a  \n",
            " 17  FHED  23     0xfe5d  \n",
            " 18  FDAT  40     0xfe80  \n",
            " 20  FHED  22     0xfec0  \n",
            " 21  FDAT  54167  0xfee2  \n",
            " 23  FHED  26     0x1d291 \n",
            " 24  FDAT  8      0x1d2b7 \n",
            " 26  FHED  18     0x1d2d7 \n",
            " 27  FDAT  18     0x1d2f5 \n",
        ));
}

/// Precondition: A deflate-compressed PNA archive exists.
/// Action: Run `pna experimental chunk list --type FHED --exclude-type FHED -f <archive>`.
/// Expectation: No rows are emitted because exclusions take precedence.
#[test]
fn chunk_list_exclude_overrides_include() {
    setup();
    TestResources::extract_in("deflate.pna", "chunk_list_type_exclude_priority/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--type",
            "FHED",
            "--exclude-type",
            "FHED",
            "-f",
            "chunk_list_type_exclude_priority/deflate.pna",
        ])
        .assert()
        .success()
        .stdout("\n");
}

/// Precondition: A deflate-compressed PNA archive exists.
/// Action: Run `pna experimental chunk list --type FHED -f <archive>`.
/// Expectation: Index and Offset columns preserve archive positions, not display order.
#[test]
fn chunk_list_type_preserves_index_and_offset() {
    setup();
    TestResources::extract_in("deflate.pna", "chunk_list_type_index_offset/").unwrap();

    let output = cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "--type",
            "FHED",
            "-f",
            "chunk_list_type_index_offset/deflate.pna",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);
    assert!(
        output_str.contains(" 2   FHED"),
        "first FHED row should keep archive index 2, got:\n{output_str}"
    );
    assert!(
        output_str.contains("0x001c"),
        "first FHED row should keep archive offset 0x001c, got:\n{output_str}"
    );
    assert!(
        !output_str.contains(" 1   FHED"),
        "filtered rows must not be renumbered, got:\n{output_str}"
    );
}

/// Precondition: A deflate-compressed PNA archive exists.
/// Action: Run chunk list with an invalid --type value.
/// Expectation: Argument parsing fails with a descriptive error.
#[test]
fn chunk_list_with_invalid_type_length() {
    setup();
    TestResources::extract_in("deflate.pna", "chunk_list_invalid_type_len/").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "experimental",
        "chunk",
        "list",
        "--type",
        "toolong",
        "-f",
        "chunk_list_invalid_type_len/deflate.pna",
    ]);

    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("chunk type must be exactly 4 ASCII characters"),
        "unexpected error message: {err}"
    );
}

/// Precondition: A deflate-compressed PNA archive exists.
/// Action: Run chunk list with a non-alphabetic --type value.
/// Expectation: Argument parsing fails with a descriptive error.
#[test]
fn chunk_list_with_invalid_type_chars() {
    setup();
    TestResources::extract_in("deflate.pna", "chunk_list_invalid_type_chars/").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "experimental",
        "chunk",
        "list",
        "--type",
        "FH1D",
        "-f",
        "chunk_list_invalid_type_chars/deflate.pna",
    ]);

    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("chunk type must contain only ASCII alphabetic characters"),
        "unexpected error message: {err}"
    );
}

/// Precondition: An empty PNA archive exists.
/// Action: Run `pna experimental chunk list -f <archive>` without type filters.
/// Expectation: All chunks are listed as before.
#[test]
fn chunk_list_without_type_filter() {
    setup();
    TestResources::extract_in("empty.pna", "chunk_list_no_type_filter/").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "chunk",
            "list",
            "-f",
            "chunk_list_no_type_filter/empty.pna",
        ])
        .assert()
        .success()
        .stdout(concat!(" 1  AHED  8  0x0008 \n", " 2  AEND  0  0x001c \n",));
}
