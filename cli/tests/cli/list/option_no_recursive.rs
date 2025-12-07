use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;

/// Precondition: An archive contains entries under 'raw/images/' directory.
/// Action: Run `pna list` with 'raw/images' as positional argument (default recursive).
/// Expectation: All entries under 'raw/images/' are listed due to prefix matching.
#[test]
fn list_recursive_by_default() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_recursive_default/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-f",
        "list_recursive_default/zstd_keep_all.pna",
        "raw/images",
    ])
    .assert()
    // Without -n, matches directory and all children
    .success()
    .stdout(concat!(
        "raw/images\n",
        "raw/images/icon.svg\n",
        "raw/images/icon.png\n",
        "raw/images/icon.bmp\n",
    ));
}

/// Precondition: An archive contains entries under 'raw/images/' directory.
/// Action: Run `pna list -n` with 'raw/images' as positional argument.
/// Expectation: Only the exact 'raw/images' directory entry is listed, not its children.
#[test]
fn list_no_recursive_matches_exact_only() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_no_recursive_exact/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-n",
        "-f",
        "list_no_recursive_exact/zstd_keep_all.pna",
        "raw/images",
    ])
    .assert()
    // With -n, only matches the exact entry, not children
    .success()
    .stdout("raw/images\n");
}

/// Precondition: An archive contains entries under 'raw/images/' directory.
/// Action: Run `pna list --no-recursive` (long form) with 'raw/images' as positional argument.
/// Expectation: Only the exact 'raw/images' directory entry is listed.
#[test]
fn list_no_recursive_long_form() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_no_recursive_long/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "--no-recursive",
        "-f",
        "list_no_recursive_long/zstd_keep_all.pna",
        "raw/images",
    ])
    .assert()
    .success()
    .stdout("raw/images\n");
}

/// Precondition: An archive contains multiple file entries.
/// Action: Run `pna list -n` with exact file path.
/// Expectation: Exact file path still matches.
#[test]
fn list_no_recursive_exact_file_path() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_no_recursive_file/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-n",
        "-f",
        "list_no_recursive_file/zstd_keep_all.pna",
        "raw/text.txt",
    ])
    .assert()
    .success()
    .stdout("raw/text.txt\n");
}

/// Precondition: An archive contains entries like 'raw/images/icon.png'.
/// Action: Run `pna list -n` with glob pattern 'raw/images/*.png'.
/// Expectation: Glob patterns still work with -n flag.
#[test]
fn list_no_recursive_glob_still_works() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_no_recursive_glob/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-n",
        "-f",
        "list_no_recursive_glob/zstd_keep_all.pna",
        "raw/images/*.png",
    ])
    .assert()
    // Glob patterns work regardless of -n
    .success()
    .stdout("raw/images/icon.png\n");
}

/// Precondition: An archive contains entries.
/// Action: Run `pna list -n` with multiple patterns.
/// Expectation: Multiple exact patterns work, no prefix expansion.
#[test]
fn list_no_recursive_multiple_patterns() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_no_recursive_multi/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-n",
        "-f",
        "list_no_recursive_multi/zstd_keep_all.pna",
        "raw/images",
        "raw/pna",
    ])
    .assert()
    // Only exact directory entries, no children
    .success()
    .stdout(concat!("raw/images\n", "raw/pna\n",));
}

/// Precondition: An archive contains entries.
/// Action: Run `pna list -n` with a pattern that doesn't match exactly.
/// Expectation: Entry not found because prefix matching is disabled.
#[test]
fn list_no_recursive_unmatched_prefix() {
    setup();
    TestResources::extract_in("zstd_with_raw_file_size.pna", "list_no_recursive_unmatch/").unwrap();

    // This archive has raw/images/icon.png but no 'raw/images' directory entry
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-n",
        "-f",
        "list_no_recursive_unmatch/zstd_with_raw_file_size.pna",
        "raw/images",
    ])
    .assert()
    // Should fail because 'raw/images' exact entry doesn't exist in this archive
    .failure();
}

/// Precondition: An archive contains 'raw/images' directory entry.
/// Action: Run `pna list` with partial segment pattern 'raw/im'.
/// Expectation: Pattern does NOT match 'raw/images' because 'im' is not a complete path segment.
#[test]
fn list_partial_segment_no_match() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_partial_segment/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-f",
        "list_partial_segment/zstd_keep_all.pna",
        "raw/im",
    ])
    .assert()
    // 'raw/im' should NOT match 'raw/images' - partial segment matching is not allowed
    .failure();
}

/// Precondition: An archive contains entries under 'raw/images/' directory.
/// Action: Run `pna list` with trailing slash pattern 'raw/images/'.
/// Expectation: Trailing slash is normalized, matches same as 'raw/images'.
#[test]
fn list_trailing_slash_pattern() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_trailing_slash/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-f",
        "list_trailing_slash/zstd_keep_all.pna",
        "raw/images/",
    ])
    .assert()
    // Trailing slash should be normalized, matching directory and children
    .success()
    .stdout(concat!(
        "raw/images\n",
        "raw/images/icon.svg\n",
        "raw/images/icon.png\n",
        "raw/images/icon.bmp\n",
    ));
}

/// Precondition: An archive contains entries.
/// Action: Compare output with and without -n for same pattern.
/// Expectation: Without -n, more entries are matched via prefix expansion.
#[test]
fn list_recursive_vs_no_recursive() {
    setup();
    TestResources::extract_in("zstd_keep_all.pna", "list_recursive_compare/").unwrap();

    // Get output without -n (recursive)
    let mut cmd1 = cargo_bin_cmd!("pna");
    let output_recursive = cmd1
        .args([
            "list",
            "-f",
            "list_recursive_compare/zstd_keep_all.pna",
            "raw/pna",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Get output with -n (no-recursive)
    let mut cmd2 = cargo_bin_cmd!("pna");
    let output_no_recursive = cmd2
        .args([
            "list",
            "-n",
            "-f",
            "list_recursive_compare/zstd_keep_all.pna",
            "raw/pna",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    // Recursive should have more entries
    let recursive_lines: Vec<&str> = std::str::from_utf8(&output_recursive)
        .unwrap()
        .lines()
        .collect();
    let no_recursive_lines: Vec<&str> = std::str::from_utf8(&output_no_recursive)
        .unwrap()
        .lines()
        .collect();

    assert!(
        recursive_lines.len() > no_recursive_lines.len(),
        "recursive should match more entries than no-recursive"
    );
    assert_eq!(
        no_recursive_lines.len(),
        1,
        "no-recursive should match exactly 1 entry"
    );
}
