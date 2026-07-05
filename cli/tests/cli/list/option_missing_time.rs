use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;

fn create_archive(dir: &str, timestamp_flag: &str) {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();
    fs::write(format!("{dir}/file.txt"), "content").unwrap();
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "c",
        "-f",
        &format!("{dir}/archive.pna"),
        "--overwrite",
        &format!("{dir}/file.txt"),
        timestamp_flag,
    ])
    .assert()
    .success();
}

/// Precondition: Archive entry has no mtime (mTIM chunk omitted).
/// Action: Run `pna list` with a newer-mtime filter and `--missing-time exclude`.
/// Expectation: The mtime-missing entry is hidden.
#[test]
fn list_missing_time_exclude_hides_entry() {
    setup();
    create_archive("list_missing_time_exclude", "--no-keep-timestamp");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-f",
        "list_missing_time_exclude/archive.pna",
        "--unstable",
        "--newer-mtime",
        "@1000000000",
        "--missing-time",
        "exclude",
    ])
    .assert()
    .success()
    .stdout("");
}

/// Precondition: Archive entry has no mtime.
/// Action: Run `pna list` with a newer-mtime filter and no `--missing-time`.
/// Expectation: Default `include` policy shows the mtime-missing entry.
#[test]
fn list_missing_time_default_shows_entry() {
    setup();
    create_archive("list_missing_time_default", "--no-keep-timestamp");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-f",
        "list_missing_time_default/archive.pna",
        "--unstable",
        "--newer-mtime",
        "@1000000000",
    ])
    .assert()
    .success()
    .stdout("list_missing_time_default/file.txt\n");
}

/// Precondition: Archive entry has no mtime.
/// Action: Run `pna list` with a newer-mtime filter and `--missing-time epoch`.
/// Expectation: The entry is assumed infinitely old and hidden.
#[test]
fn list_missing_time_assume_epoch_hides_with_newer_filter() {
    setup();
    create_archive("list_missing_time_epoch_newer", "--no-keep-timestamp");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-f",
        "list_missing_time_epoch_newer/archive.pna",
        "--unstable",
        "--newer-mtime",
        "@1000000000",
        "--missing-time",
        "epoch",
    ])
    .assert()
    .success()
    .stdout("");
}

/// Precondition: Archive entry has no mtime.
/// Action: Run `pna list` with an older-mtime filter and `--missing-time epoch`.
/// Expectation: The entry is assumed infinitely old and shown.
#[test]
fn list_missing_time_assume_epoch_shows_with_older_filter() {
    setup();
    create_archive("list_missing_time_epoch_older", "--no-keep-timestamp");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-f",
        "list_missing_time_epoch_older/archive.pna",
        "--unstable",
        "--older-mtime",
        "@1000000000",
        "--missing-time",
        "epoch",
    ])
    .assert()
    .success()
    .stdout("list_missing_time_epoch_older/file.txt\n");
}

/// Precondition: None.
/// Action: Run `pna list --missing-time exclude` without any time-filter option.
/// Expectation: Argument parsing fails.
#[test]
fn list_missing_time_requires_time_filter() {
    setup();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-f",
        "archive.pna",
        "--unstable",
        "--missing-time",
        "exclude",
    ])
    .assert()
    .failure();
}

/// Precondition: Archive entry has no ctime.
/// Action: Run `pna list` with a ctime filter (no mtime filter) and `--missing-time exclude`.
/// Expectation: The option is accepted and the ctime-missing entry is hidden.
#[test]
fn list_missing_time_accepts_ctime_filter() {
    setup();
    create_archive("list_missing_time_ctime", "--no-keep-timestamp");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-f",
        "list_missing_time_ctime/archive.pna",
        "--unstable",
        "--newer-ctime",
        "@1000000000",
        "--missing-time",
        "exclude",
    ])
    .assert()
    .success()
    .stdout("");
}

/// Precondition: Archive entry has an mtime newer than the filter threshold.
/// Action: Run `pna list` with only a newer-mtime filter and `--missing-time exclude`.
/// Expectation: The entry is shown; the exclude policy must not apply to the
/// unbounded ctime filter even when the entry lacks ctime.
#[test]
fn list_missing_time_exclude_ignores_unbounded_ctime_filter() {
    setup();
    create_archive("list_missing_time_unbounded", "--keep-timestamp");

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "list",
        "-f",
        "list_missing_time_unbounded/archive.pna",
        "--unstable",
        "--newer-mtime",
        "@1000000000",
        "--missing-time",
        "exclude",
    ])
    .assert()
    .success()
    .stdout("list_missing_time_unbounded/file.txt\n");
}
