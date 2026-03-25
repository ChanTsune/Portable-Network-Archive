use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;

/// Precondition: A snapshot archive with a single flag (uchg).
/// Action: Run `pna experimental fflag get` to read the flag.
/// Expectation: The uchg flag is correctly read from the archive.
#[test]
fn fflag_compatibility_single_flag() {
    setup();
    TestResources::extract_in("zstd_keep_fflags.pna", ".").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "zstd_keep_fflags.pna",
            "file1.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("file1.txt"))
        .stdout(predicates::str::contains("uchg"));
}

/// Precondition: A snapshot archive with multiple flags (uchg, nodump, hidden).
/// Action: Run `pna experimental fflag get` to read the flags.
/// Expectation: All flags are correctly read from the archive.
#[test]
fn fflag_compatibility_multi_flag() {
    setup();
    TestResources::extract_in("zstd_keep_fflags.pna", ".").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "zstd_keep_fflags.pna",
            "testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("testfile.txt"))
        .stdout(predicates::str::contains("uchg"))
        .stdout(predicates::str::contains("nodump"))
        .stdout(predicates::str::contains("hidden"));
}

/// Precondition: A snapshot archive with multiple entries having different flags.
/// Action: Run `pna experimental fflag get` to read all flags.
/// Expectation: Each entry's flags are correctly read.
#[test]
fn fflag_compatibility_multi_entry() {
    setup();
    TestResources::extract_in("zstd_keep_fflags.pna", ".").unwrap();

    let output = cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "zstd_keep_fflags.pna",
            "--long",
            "*",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_str = String::from_utf8_lossy(&output);

    // Verify file1.txt has uchg
    assert!(output_str.contains("file1.txt"));
    assert!(output_str.contains("uchg"));

    // Verify file2.txt has nodump
    assert!(output_str.contains("file2.txt"));
    assert!(output_str.contains("nodump"));

    // Verify file3.txt has hidden and schg
    assert!(output_str.contains("file3.txt"));
    assert!(output_str.contains("hidden"));
    assert!(output_str.contains("schg"));
}

/// Precondition: A snapshot archive with fflags.
/// Action: Run `pna experimental fflag get --dump` to output restorable format.
/// Expectation: Output is in the correct dump format.
#[test]
fn fflag_compatibility_dump_format() {
    setup();
    TestResources::extract_in("zstd_keep_fflags.pna", ".").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "zstd_keep_fflags.pna",
            "--dump",
            "testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("# file: testfile.txt"))
        .stdout(predicates::str::contains("flags="));
}

/// Precondition: A snapshot archive with fflags.
/// Action: Run `pna experimental fflag get --name <flag>` to filter.
/// Expectation: Only entries with the specified flag are shown.
#[test]
fn fflag_compatibility_filter_by_name() {
    setup();
    TestResources::extract_in("zstd_keep_fflags.pna", ".").unwrap();

    // Filter for schg - should only show file3.txt
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "zstd_keep_fflags.pna",
            "--name",
            "schg",
            "*",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("file3.txt"))
        .stdout(predicates::str::contains("file1.txt").not())
        .stdout(predicates::str::contains("file2.txt").not())
        .stdout(predicates::str::contains("testfile.txt").not());
}

/// Precondition: A snapshot archive with fflags.
/// Action: Set additional flags on the snapshot archive.
/// Expectation: New flags are added without removing existing ones.
#[test]
fn fflag_compatibility_add_flags() {
    setup();
    TestResources::extract_in("zstd_keep_fflags.pna", "fflag_compat_add_flags/").unwrap();

    // Add nodump to existing uchg on file1
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_compat_add_flags/zstd_keep_fflags.pna",
            "nodump",
            "file1.txt",
        ])
        .assert()
        .success();

    // Verify both flags are present
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_compat_add_flags/zstd_keep_fflags.pna",
            "file1.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg"))
        .stdout(predicates::str::contains("nodump"));
}
