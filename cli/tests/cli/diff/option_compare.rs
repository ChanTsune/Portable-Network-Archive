use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;
use predicates::prelude::*;
use std::fs;

/// Lays out `store.pna`, which predates the fSIZ chunk, next to the tree it was
/// built from, so that diff resolves every entry instead of reporting it missing.
fn place_archive_without_recorded_sizes(dir: &str) -> &str {
    let _ = fs::remove_dir_all(dir);
    TestResources::extract_in("raw/", dir).unwrap();
    TestResources::extract_in("store.pna", dir).unwrap();
    dir
}

/// Archives a file, then rewrites it either at its original length or longer.
fn create_archive_with_resized_file(dir: &str, resized: bool) -> (String, String) {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "old-a").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file_path])
        .assert()
        .success();

    let updated = if resized {
        "a much longer content"
    } else {
        "new-a"
    };
    fs::write(&file_path, updated).unwrap();

    (archive_path, file_path)
}

/// Archives a directory and the file inside it, then changes both the file
/// content and the directory mtime.
fn create_archive_with_retimed_directory(dir: &str) -> (String, String) {
    use std::time::{Duration, SystemTime};

    let _ = fs::remove_dir_all(dir);
    let subdir = format!("{dir}/subdir");
    fs::create_dir_all(&subdir).unwrap();

    let file_path = format!("{subdir}/file.txt");
    fs::write(&file_path, "old-a").unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            "--keep-timestamp",
            "--keep-dir",
            &subdir,
        ])
        .assert()
        .success();

    fs::write(&file_path, "new-a").unwrap();
    let new_mtime = SystemTime::UNIX_EPOCH + Duration::from_secs(86400);
    filetime::set_file_mtime(&subdir, filetime::FileTime::from_system_time(new_mtime)).unwrap();

    (archive_path, file_path)
}

/// Precondition: Archive contains a file whose content changed without changing its size.
/// Action: Run diff with `--compare size`.
/// Expectation: The content difference is not reported, because size was the only selected field.
#[test]
fn diff_with_compare_size_ignores_content() {
    setup();
    let (archive_path, _) =
        create_archive_with_resized_file("diff_compare_size_ignores_test", false);

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "size",
            "--unstable",
        ])
        .assert()
        .success()
        .stderr("")
        .stdout("");
}

/// Precondition: Archive contains a file that grew on disk.
/// Action: Run diff with `--compare size`.
/// Expectation: The size difference is reported.
#[test]
fn diff_with_compare_size_detects_size_change() {
    setup();
    let (archive_path, _) =
        create_archive_with_resized_file("diff_compare_size_detects_test", true);

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "size",
            "--unstable",
        ])
        .assert()
        .code(1)
        .stderr("")
        .stdout(predicate::str::contains("Size differs"));
}

/// Precondition: Archive contains a file that grew on disk.
/// Action: Run diff with `--compare content`, leaving size unselected.
/// Expectation: The difference is reported as a content difference, not a size difference.
#[test]
fn diff_with_compare_content_reports_size_change_as_content() {
    setup();
    let (archive_path, file_path) =
        create_archive_with_resized_file("diff_compare_content_substitution_test", true);

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "content",
            "--unstable",
            "--format",
            "jsonl",
        ])
        .assert()
        .code(1)
        .stderr("")
        .stdout(format!(r#"{{"path":"{file_path}","kind":"content"}}"#) + "\n");
}

/// Precondition: Archive contains a file whose content changed without changing its size.
/// Action: Run diff with `--compare default`.
/// Expectation: Output is identical to running diff without `--compare`.
#[test]
fn diff_with_compare_default_matches_the_omitted_option() {
    setup();
    let (archive_path, _) =
        create_archive_with_resized_file("diff_compare_default_equivalence_test", false);

    let implicit = cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .code(1)
        .get_output()
        .stdout
        .clone();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "default",
            "--unstable",
        ])
        .assert()
        .code(1)
        .stderr("")
        .stdout(implicit);
}

/// Precondition: Archive stores a directory whose mtime changed on disk.
/// Action: Run diff with `--compare mtime`.
/// Expectation: The directory mtime difference is reported, which the default profile suppresses.
#[test]
fn diff_with_compare_mtime_covers_directories() {
    setup();
    let (archive_path, _) =
        create_archive_with_retimed_directory("diff_compare_directory_mtime_test");

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "mtime",
            "--unstable",
        ])
        .assert()
        .code(1)
        .stderr("")
        .stdout(predicate::str::contains("Mod time differs"));
}

/// Precondition: Archive stores a directory whose mtime changed and a file whose content changed.
/// Action: Run diff with `--compare default,mtime`.
/// Expectation: The default profile still applies and mtime is compared in addition to it.
#[test]
fn diff_with_compare_default_adds_to_the_selected_fields() {
    setup();
    let (archive_path, _) =
        create_archive_with_retimed_directory("diff_compare_default_union_test");

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "default,mtime",
            "--unstable",
        ])
        .assert()
        .code(1)
        .stderr("")
        .stdout(
            predicate::str::contains("Contents differ")
                .and(predicate::str::contains("Mod time differs")),
        );
}

/// Precondition: None.
/// Action: Parse a diff invocation combining `--compare` and `--full-compare`.
/// Expectation: Parsing fails with a mutual exclusion error.
#[test]
fn diff_with_compare_conflicts_with_full_compare() {
    setup();

    let result = cli::Cli::try_parse_from([
        "pna",
        "experimental",
        "diff",
        "-f",
        "dummy.pna",
        "--compare",
        "size",
        "--full-compare",
        "--unstable",
    ]);

    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("cannot be used with"),
        "expected a mutual exclusion error, got: {err}"
    );
}

/// Precondition: None.
/// Action: Parse a diff invocation using `--compare` without `--unstable`.
/// Expectation: Parsing fails because the option is gated behind `--unstable`.
#[test]
fn diff_with_compare_requires_unstable() {
    setup();

    let result = cli::Cli::try_parse_from([
        "pna",
        "experimental",
        "diff",
        "-f",
        "dummy.pna",
        "--compare",
        "size",
    ]);

    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("--unstable"),
        "expected the unstable requirement to be reported, got: {err}"
    );
}

/// Precondition: Archive contains a path that no longer exists on disk.
/// Action: Run diff with `--compare mtime`, which selects neither missing paths nor types.
/// Expectation: The missing path is still reported, because it is not gated by field selection.
#[test]
fn diff_with_compare_still_reports_missing_paths() {
    setup();
    let (archive_path, file_path) =
        create_archive_with_resized_file("diff_compare_missing_test", false);
    fs::remove_file(&file_path).unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "mtime",
            "--unstable",
        ])
        .assert()
        .code(1)
        .stdout(predicate::str::contains("Cannot stat"));
}

/// Precondition: Archive contains a file that grew on disk.
/// Action: Run diff with `--compare uid`, which no non-Unix platform supports.
/// Expectation: The command fails instead of reporting that nothing differs.
#[cfg(not(unix))]
#[test]
fn diff_with_compare_fails_when_no_field_is_supported() {
    setup();
    let (archive_path, _) = create_archive_with_resized_file("diff_compare_unsupported_test", true);

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "uid",
            "--unstable",
        ])
        .assert()
        .code(2)
        .stderr(
            predicate::str::contains("cannot compare the requested field(s)").and(
                predicate::str::contains("uid: unsupported on this platform"),
            ),
        );
}

/// Precondition: Archive records no size for its entries, and the files match on disk.
/// Action: Run diff with `--compare size`.
/// Expectation: The command fails instead of reporting that nothing differs.
#[test]
fn diff_with_compare_size_fails_when_no_size_was_recorded() {
    setup();
    let dir = place_archive_without_recorded_sizes("diff_compare_no_recorded_size_test");

    cargo_bin_cmd!("pna")
        .current_dir(dir)
        .args([
            "experimental",
            "diff",
            "-f",
            "store.pna",
            "--compare",
            "size",
            "--unstable",
        ])
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains(
            "size: not recorded in the archive",
        ));
}

/// Precondition: Archive records no size for its entries, and the files match on disk.
/// Action: Run diff without `--compare`.
/// Expectation: The run succeeds, because the default profile skips what it cannot compare.
#[test]
fn diff_without_compare_tolerates_missing_sizes() {
    setup();
    let dir = place_archive_without_recorded_sizes("diff_compare_default_tolerates_test");

    cargo_bin_cmd!("pna")
        .current_dir(dir)
        .args(["experimental", "diff", "-f", "store.pna"])
        .assert()
        .success()
        .stdout("");
}

/// Precondition: Archive stores a directory without a modification time.
/// Action: Run diff with `--compare mtime`.
/// Expectation: The directory entry is counted as uncomparable, not silently skipped.
#[test]
fn diff_with_compare_mtime_fails_on_directory_without_recorded_mtime() {
    setup();
    let dir = "diff_compare_directory_no_mtime_test";
    let _ = fs::remove_dir_all(dir);
    let subdir = format!("{dir}/subdir");
    fs::create_dir_all(&subdir).unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            "--keep-dir",
            &subdir,
        ])
        .assert()
        .success();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "diff",
            "-f",
            &archive_path,
            "--compare",
            "mtime",
            "--unstable",
        ])
        .assert()
        .code(2)
        .stderr(predicate::str::contains(
            "mtime: not recorded in the archive",
        ));
}

/// Precondition: Archive stores a file's mode, and the file became unreadable on disk.
/// Action: Run diff without `--compare`, so that reading the contents fails.
/// Expectation: The mode difference is still reported before the read error aborts the run.
#[cfg(unix)]
#[test]
fn diff_reports_metadata_differences_before_a_read_failure() {
    use std::os::unix::fs::PermissionsExt;

    setup();
    // SAFETY: geteuid() only reads the calling process's effective user id.
    if unsafe { libc::geteuid() } == 0 {
        return; // root can open a 0o000 file, so the read never fails.
    }

    let dir = "diff_compare_metadata_before_read_error_test";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_path = format!("{dir}/file.txt");
    fs::write(&file_path, "content").unwrap();
    fs::set_permissions(&file_path, fs::Permissions::from_mode(0o644)).unwrap();

    let archive_path = format!("{dir}/test.pna");
    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            "--keep-permission",
            &file_path,
        ])
        .assert()
        .success();

    fs::set_permissions(&file_path, fs::Permissions::from_mode(0o000)).unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "diff", "-f", &archive_path])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("Mode differs"));
}
