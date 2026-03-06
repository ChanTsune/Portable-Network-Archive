use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, Duration, EntryBuilder, EntryName, Permission, WriteOptions};
use std::{fs, io::Write, path::Path};

fn create_archive_with_trailing_slash_dir(path: &str) {
    fs::create_dir_all(Path::new(path).parent().unwrap()).unwrap();
    let file = fs::File::create(path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();

    // Jan 26 2025 00:00:00 UTC — deterministic timestamp for exact output matching
    let mtime = Duration::new(1737849600, 0);

    let mut dir_builder = EntryBuilder::new_dir(EntryName::from_utf8_preserve_root("dir/"));
    dir_builder.modified(mtime).permission(Permission::new(
        0,
        "root".into(),
        0,
        "root".into(),
        0o755,
    ));
    archive.add_entry(dir_builder.build().unwrap()).unwrap();

    let mut file_builder = EntryBuilder::new_file(
        EntryName::from_utf8_preserve_root("dir/file.txt"),
        WriteOptions::store(),
    )
    .unwrap();
    file_builder.modified(mtime).permission(Permission::new(
        0,
        "root".into(),
        0,
        "root".into(),
        0o644,
    ));
    file_builder.write_all(b"hello").unwrap();
    archive.add_entry(file_builder.build().unwrap()).unwrap();

    archive.finalize().unwrap();
}

/// Precondition: An archive contains a directory entry whose stored name ends with '/'.
/// Action: Run `pna list --classify`.
/// Expectation: The directory path has exactly one trailing slash, not two.
#[test]
fn list_classify_directory_no_duplicate_trailing_slash() {
    setup();
    create_archive_with_trailing_slash_dir("trailing_slash_classify/archive.pna");

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--classify",
            "-f",
            "trailing_slash_classify/archive.pna",
        ])
        .assert()
        .success();

    assert.stdout("dir/\ndir/file.txt\n");
}

/// Precondition: An archive contains a directory entry whose stored name ends with '/'.
/// Action: Run `pna list --format bsdtar`.
/// Expectation: The directory name in bsdtar format has exactly one trailing slash.
#[test]
fn list_bsdtar_directory_no_duplicate_trailing_slash() {
    setup();
    create_archive_with_trailing_slash_dir("trailing_slash_bsdtar/archive.pna");

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "--format",
            "bsdtar",
            "-f",
            "trailing_slash_bsdtar/archive.pna",
            "--unstable",
        ])
        .assert()
        .success();

    assert.stdout(concat!(
        "drwxr-xr-x  0 root   root        0 Jan 26  2025 dir/\n",
        "-rw-r--r--  0 root   root        5 Jan 26  2025 dir/file.txt\n",
    ));
}

/// Precondition: An archive contains a directory entry whose stored name ends with '/'.
/// Action: Run `pna list --long --classify`.
/// Expectation: The directory path has exactly one trailing slash.
#[test]
fn list_long_classify_directory_no_duplicate_trailing_slash() {
    setup();
    create_archive_with_trailing_slash_dir("trailing_slash_long/archive.pna");

    let mut cmd = cargo_bin_cmd!("pna");
    let assert = cmd
        .args([
            "list",
            "-l",
            "--classify",
            "-f",
            "trailing_slash_long/archive.pna",
        ])
        .assert()
        .success();

    assert.stdout(concat!(
        "- - drwxr-xr-x  - 0 root root Jan 26  2025 dir/         \n",
        "- - .rw-r--r--  5 5 root root Jan 26  2025 dir/file.txt \n",
    ));
}
