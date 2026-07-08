#![cfg(not(target_family = "wasm"))]
use crate::utils::{archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::{
    fs,
    io::Write,
    time::{Duration, SystemTime},
};

const DURATION_24_HOURS: Duration = Duration::from_secs(24 * 60 * 60);

/// Precondition: An archive contains an entry created from a single file.
/// Action: Run `pna experimental update --sync` naming that same file twice
/// while it has a newer mtime than the archived entry.
/// Expectation: No duplicate-target warning is emitted (the same path
/// specified twice is treated as idempotent), and the archive ends up with
/// a single, correctly updated entry.
#[test]
fn update_with_duplicate_source_argument_is_idempotent() {
    setup();
    let dir = "update_with_duplicate_source_argument_is_idempotent";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file = format!("{dir}/a.txt");
    let archive_path = format!("{dir}/archive.pna");
    fs::write(&file, b"original").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            "--keep-timestamp",
            &file,
        ])
        .assert()
        .success();

    let threshold = SystemTime::now();
    let mut handle = fs::File::options()
        .write(true)
        .truncate(true)
        .open(&file)
        .unwrap();
    handle.write_all(b"updated").unwrap();
    handle.set_modified(threshold + DURATION_24_HOURS).unwrap();
    drop(handle);

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "update",
            "-f",
            &archive_path,
            "--sync",
            "--keep-timestamp",
            &file,
            &file,
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Multiple update sources").not());

    assert_eq!(
        archive::get_archive_entry_names(&archive_path),
        vec![file.clone()],
        "duplicate source argument should collapse to a single archive entry"
    );

    let out_dir = format!("{dir}/out");
    cargo_bin_cmd!("pna")
        .args([
            "x",
            "-f",
            &archive_path,
            "--overwrite",
            "--out-dir",
            &out_dir,
        ])
        .assert()
        .success();
    assert_eq!(
        fs::read_to_string(format!("{out_dir}/{file}")).unwrap(),
        "updated"
    );
}

/// Precondition: An archive contains a directory and a file inside it.
/// Action: Run `pna experimental update --sync` naming both the directory
/// and the contained file while the file has a newer mtime than the archive.
/// Expectation: No duplicate-target warning is emitted because the repeated
/// collected path came from overlapping source arguments and is idempotent.
#[test]
fn update_with_overlapping_directory_and_file_arguments_does_not_warn() {
    setup();
    let dir = "update_with_overlapping_directory_and_file_arguments_does_not_warn";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file = format!("{dir}/a.txt");
    let archive_path = format!("{dir}/archive.pna");
    fs::write(&file, b"original").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            "--keep-timestamp",
            dir,
        ])
        .assert()
        .success();

    let threshold = SystemTime::now();
    let mut handle = fs::File::options()
        .write(true)
        .truncate(true)
        .open(&file)
        .unwrap();
    handle.write_all(b"updated").unwrap();
    handle.set_modified(threshold + DURATION_24_HOURS).unwrap();
    drop(handle);

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "update",
            "-f",
            &archive_path,
            "--sync",
            "--keep-timestamp",
            dir,
            &file,
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Multiple update sources").not());
}

/// Precondition: An archive contains multiple entries.
/// Action: Run a normal `pna experimental update` with no overlapping or
/// colliding source arguments.
/// Expectation: No duplicate-target warning is emitted.
#[test]
fn update_without_duplicate_targets_does_not_warn() {
    setup();
    let dir = "update_without_duplicate_targets_does_not_warn";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file_a = format!("{dir}/a.txt");
    let file_b = format!("{dir}/b.txt");
    let archive_path = format!("{dir}/archive.pna");
    fs::write(&file_a, b"a").unwrap();
    fs::write(&file_b, b"b").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "create",
            "-f",
            &archive_path,
            "--overwrite",
            &file_a,
            &file_b,
        ])
        .assert()
        .success();

    fs::write(&file_a, b"a-updated").unwrap();

    cargo_bin_cmd!("pna")
        .args(["experimental", "update", "-f", &archive_path, &file_a])
        .assert()
        .success()
        .stderr(predicate::str::contains("Multiple update sources").not());
}

/// Precondition: An archive contains an entry created from a single file.
/// Action: Run `pna experimental update` naming that file twice with
/// different literal spellings ("dir/a.txt" and "./dir/a.txt") that resolve
/// to the same archive entry name.
/// Expectation: A warning is emitted identifying the colliding archive entry
/// since two distinct source paths - not merely the same path repeated -
/// collapsed onto it.
#[test]
fn update_with_conflicting_source_spellings_warns() {
    setup();
    let dir = "update_with_conflicting_source_spellings_warns";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let file = format!("{dir}/a.txt");
    let file_dotted = format!("./{dir}/a.txt");
    let archive_path = format!("{dir}/archive.pna");
    fs::write(&file, b"content").unwrap();

    cargo_bin_cmd!("pna")
        .args(["create", "-f", &archive_path, "--overwrite", &file])
        .assert()
        .success();

    fs::write(&file, b"updated").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "update",
            "-f",
            &archive_path,
            &file,
            &file_dotted,
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(format!(
            "Multiple update sources map to the same archive entry \"{file}\": \"{file_dotted}\" is used, \"{file}\" is discarded"
        )));
}
