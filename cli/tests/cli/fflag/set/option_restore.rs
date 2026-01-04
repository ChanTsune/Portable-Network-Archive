use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::io::Write;

/// Precondition: A dump file with flags for multiple entries.
/// Action: Run `pna experimental fflag set --restore` to restore flags.
/// Expectation: Flags are restored from the dump file.
#[test]
fn fflag_set_restore_from_dump() {
    setup();
    fs::create_dir_all("fflag_restore").unwrap();

    fs::write("fflag_restore/file1.txt", "content 1").unwrap();
    fs::write("fflag_restore/file2.txt", "content 2").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_restore/test.pna",
            "--overwrite",
            "fflag_restore/file1.txt",
            "fflag_restore/file2.txt",
        ])
        .assert()
        .success();

    // Create a dump file
    let dump_content = "# file: fflag_restore/file1.txt\nflags=uchg,nodump\n\n# file: fflag_restore/file2.txt\nflags=hidden\n";
    fs::write("fflag_restore/flags.dump", dump_content).unwrap();

    // Restore from dump
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_restore/test.pna",
            "--restore",
            "fflag_restore/flags.dump",
        ])
        .assert()
        .success();

    // Verify file1 has uchg and nodump
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_restore/test.pna",
            "fflag_restore/file1.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg"))
        .stdout(predicates::str::contains("nodump"));

    // Verify file2 has hidden
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_restore/test.pna",
            "fflag_restore/file2.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("hidden"));
}

/// Precondition: An archive with flags set.
/// Action: Dump flags and restore them to another archive.
/// Expectation: Round-trip preserves flags exactly.
#[test]
fn fflag_dump_and_restore_roundtrip() {
    setup();
    fs::create_dir_all("fflag_roundtrip").unwrap();

    fs::write("fflag_roundtrip/testfile.txt", "test content").unwrap();

    // Create source archive
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_roundtrip/source.pna",
            "--overwrite",
            "fflag_roundtrip/testfile.txt",
        ])
        .assert()
        .success();

    // Set flags on source
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_roundtrip/source.pna",
            "uchg,nodump,hidden",
            "fflag_roundtrip/testfile.txt",
        ])
        .assert()
        .success();

    // Dump flags to file
    let output = cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_roundtrip/source.pna",
            "--dump",
            "*",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let mut dump_file = fs::File::create("fflag_roundtrip/flags.dump").unwrap();
    dump_file.write_all(&output).unwrap();

    // Create target archive
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_roundtrip/target.pna",
            "--overwrite",
            "fflag_roundtrip/testfile.txt",
        ])
        .assert()
        .success();

    // Restore flags to target
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_roundtrip/target.pna",
            "--restore",
            "fflag_roundtrip/flags.dump",
        ])
        .assert()
        .success();

    // Verify target has same flags as source
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "get",
            "-f",
            "fflag_roundtrip/target.pna",
            "fflag_roundtrip/testfile.txt",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("uchg"))
        .stdout(predicates::str::contains("nodump"))
        .stdout(predicates::str::contains("hidden"));
}

/// Precondition: A dump file with flags for an entry that doesn't exist.
/// Action: Run `pna experimental fflag set --restore`.
/// Expectation: Command succeeds silently ignoring missing entries.
#[test]
fn fflag_restore_missing_entry() {
    setup();
    fs::create_dir_all("fflag_restore_missing").unwrap();

    fs::write("fflag_restore_missing/file.txt", "content").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "fflag_restore_missing/test.pna",
            "--overwrite",
            "fflag_restore_missing/file.txt",
        ])
        .assert()
        .success();

    // Create dump file referencing non-existent entry
    let dump_content = "# file: nonexistent.txt\nflags=uchg\n";
    fs::write("fflag_restore_missing/flags.dump", dump_content).unwrap();

    // Restore succeeds but silently ignores missing entries
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "fflag",
            "set",
            "-f",
            "fflag_restore_missing/test.pna",
            "--restore",
            "fflag_restore_missing/flags.dump",
        ])
        .assert()
        .success();
}
