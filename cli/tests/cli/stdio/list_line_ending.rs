#![cfg(not(target_family = "wasm"))]

use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, WriteOptions};
use std::io::Write;

fn build_two_entry_archive() -> Vec<u8> {
    let mut archive = Archive::write_header(Vec::new()).unwrap();

    let mut a = EntryBuilder::new_file("a.txt".into(), WriteOptions::store()).unwrap();
    a.write_all(b"alpha").unwrap();
    archive.add_entry(a.build().unwrap()).unwrap();

    let mut b = EntryBuilder::new_file("b.txt".into(), WriteOptions::store()).unwrap();
    b.write_all(b"beta").unwrap();
    archive.add_entry(b.build().unwrap()).unwrap();

    archive.finalize().unwrap()
}

#[cfg(target_os = "windows")]
const EXPECTED_LINE_ENDING: &[u8] = b"\r\n";

#[cfg(not(target_os = "windows"))]
const EXPECTED_LINE_ENDING: &[u8] = b"\n";

/// Precondition: An archive with multiple entries is piped through stdin.
/// Action: Run bsdtar-compat list (simple format) reading from stdin.
/// Expectation: Every record ends with the platform's expected line
///   separator (CRLF on Windows, LF elsewhere) so the output matches
///   the byte stream that the reference bsdtar implementation produces.
#[test]
fn stdio_list_simple_uses_platform_line_ending() {
    setup();
    let archive_data = build_two_entry_archive();

    let output = cargo_bin_cmd!("pna")
        .write_stdin(archive_data)
        .args(["compat", "bsdtar", "--unstable", "-tf", "-"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let separator_count = output
        .windows(EXPECTED_LINE_ENDING.len())
        .filter(|w| *w == EXPECTED_LINE_ENDING)
        .count();
    assert_eq!(
        separator_count, 2,
        "expected two records terminated by the platform line ending in {output:?}"
    );
    assert!(
        output.ends_with(EXPECTED_LINE_ENDING),
        "expected list output to end with the platform line ending: {output:?}"
    );

    #[cfg(target_os = "windows")]
    {
        // On Windows the bare LF byte must only appear as the second
        // byte of a CRLF pair so downstream tools see exact bsdtar
        // semantics rather than mixed terminators.
        let bare_lf_count = output.iter().filter(|b| **b == b'\n').count();
        assert_eq!(bare_lf_count, separator_count);
        let cr_count = output.iter().filter(|b| **b == b'\r').count();
        assert_eq!(cr_count, separator_count);
    }
}

/// Precondition: An archive with multiple entries is piped through stdin.
/// Action: Run bsdtar-compat list in verbose mode (long format).
/// Expectation: Each formatted record ends with the platform's
///   expected line separator.
#[test]
fn stdio_list_verbose_uses_platform_line_ending() {
    setup();
    let archive_data = build_two_entry_archive();

    let output = cargo_bin_cmd!("pna")
        .write_stdin(archive_data)
        .args(["compat", "bsdtar", "--unstable", "-tvf", "-"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let separator_count = output
        .windows(EXPECTED_LINE_ENDING.len())
        .filter(|w| *w == EXPECTED_LINE_ENDING)
        .count();
    assert_eq!(
        separator_count, 2,
        "expected two verbose records terminated by the platform line ending in {output:?}"
    );
    assert!(
        output.ends_with(EXPECTED_LINE_ENDING),
        "expected verbose list output to end with the platform line ending: {output:?}"
    );

    #[cfg(target_os = "windows")]
    {
        let bare_lf_count = output.iter().filter(|b| **b == b'\n').count();
        assert_eq!(bare_lf_count, separator_count);
        let cr_count = output.iter().filter(|b| **b == b'\r').count();
        assert_eq!(cr_count, separator_count);
    }
}
