use crate::utils::{archive, list_lines, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::predicate;
use std::fs;
use std::path::Path;

/// Precondition: An archive contains files in nested directories and a manifest lists a subset.
/// Action: Create the archive via `pna compat bsdtar --create`, then extract it using
///         `pna compat bsdtar --extract --files-from <manifest>`.
/// Expectation: Only the manifest entries appear in the output tree; other archive members stay
///         untouched.
#[test]
fn bsdtar_extract_with_files_from() {
    setup();

    // Prepare input payload
    let base = Path::new("bsdtar_files_from");
    let input = base.join("in");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("keep_a.txt"), "keep-a").unwrap();
    fs::write(input.join("keep_b.txt"), "keep-b").unwrap();
    fs::write(input.join("drop.txt"), "drop").unwrap();
    fs::create_dir_all(input.join("nested")).unwrap();
    fs::write(input.join("nested").join("keep_nested.txt"), "keep-nested").unwrap();

    // Create archive using bsdtar mode
    let archive_path = base.join("archive.pna");
    let mut create_cmd = cargo_bin_cmd!("pna");
    create_cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--create",
        "--unstable",
        "--overwrite",
        "-f",
        archive_path.to_str().unwrap(),
        "-C",
        input.to_str().unwrap(),
        ".",
    ]);
    create_cmd.assert().success();

    // Prepare file list (newline-delimited)
    let list_path = base.join("files.txt");
    fs::create_dir_all(base).unwrap();
    fs::write(&list_path, "keep_a.txt\nnested/keep_nested.txt\n").unwrap();

    // Extract only files referenced by --files-from
    let output_dir = base.join("out");
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--extract",
        "--unstable",
        "--files-from",
        list_path.to_str().unwrap(),
        "--out-dir",
        output_dir.to_str().unwrap(),
        "--overwrite",
        "-f",
        archive_path.to_str().unwrap(),
    ]);
    extract_cmd.assert().success();

    // Confirm only the requested files were extracted
    assert!(output_dir.join("keep_a.txt").exists());
    assert!(output_dir.join("nested").join("keep_nested.txt").exists());
    assert!(!output_dir.join("keep_b.txt").exists());
    assert!(!output_dir.join("drop.txt").exists());

    // Ensure extracted files match originals for the selected set
    assert_eq!(
        fs::read_to_string(input.join("keep_a.txt")).unwrap(),
        fs::read_to_string(output_dir.join("keep_a.txt")).unwrap()
    );
    assert_eq!(
        fs::read_to_string(input.join("nested").join("keep_nested.txt")).unwrap(),
        fs::read_to_string(output_dir.join("nested").join("keep_nested.txt")).unwrap()
    );
}

/// Precondition: Input files exist on the filesystem.
/// Action: Create the archive via `pna compat bsdtar --create -T -` with the file list on stdin.
/// Expectation: The archive contains exactly the stdin list entries.
#[test]
fn bsdtar_files_from_stdin() {
    setup();

    let base = Path::new("bsdtar_files_from_stdin");
    let input = base.join("in");
    fs::create_dir_all(input.join("nested")).unwrap();
    fs::write(input.join("keep_a.txt"), "keep-a").unwrap();
    fs::write(input.join("keep_b.txt"), "keep-b").unwrap();
    fs::write(input.join("nested").join("keep_nested.txt"), "keep-nested").unwrap();

    let archive_path = base.join("archive.pna");
    let mut create_cmd = cargo_bin_cmd!("pna");
    create_cmd
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            archive_path.to_str().unwrap(),
            "-C",
            input.to_str().unwrap(),
            "-T",
            "-",
        ])
        .write_stdin("keep_a.txt\nnested/keep_nested.txt\n");
    create_cmd.assert().success();

    let mut names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        names.push(entry.header().path().to_string());
    })
    .unwrap();
    names.sort();
    assert_eq!(names, ["keep_a.txt", "nested/keep_nested.txt"]);
}

/// Precondition: An archive contains multiple entries and a manifest lists a subset.
/// Action: List the archive via `pna compat bsdtar --list -T <manifest>`.
/// Expectation: Only the manifest entries are listed.
#[test]
fn bsdtar_list_with_files_from() {
    setup();

    let base = Path::new("bsdtar_list_with_files_from");
    let input = base.join("in");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("keep_a.txt"), "keep-a").unwrap();
    fs::write(input.join("drop.txt"), "drop").unwrap();

    let manifest_path = base.join("files.txt");
    fs::create_dir_all(base).unwrap();
    fs::write(&manifest_path, "keep_a.txt\ndrop.txt\n").unwrap();

    let archive_path = base.join("archive.pna");
    let mut create_cmd = cargo_bin_cmd!("pna");
    create_cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--create",
        "--unstable",
        "--overwrite",
        "-f",
        archive_path.to_str().unwrap(),
        "-C",
        input.to_str().unwrap(),
        "-T",
        manifest_path.to_str().unwrap(),
    ]);
    create_cmd.assert().success();

    let subset_path = base.join("subset.txt");
    fs::write(&subset_path, "keep_a.txt\n").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "compat",
            "bsdtar",
            "--list",
            "--unstable",
            "-T",
            subset_path.to_str().unwrap(),
            "-f",
            archive_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(list_lines(&["keep_a.txt"]));
}

/// Precondition: An archive exists and no file named `-` exists in the working directory.
/// Action: Extract via `pna compat bsdtar --extract --files-from -` with a valid pattern on stdin.
/// Expectation: The command fails instead of reading the pattern from stdin.
#[test]
fn bsdtar_extract_files_from_dash_is_filename() {
    setup();
    let _ = fs::remove_file("-");

    let base = Path::new("bsdtar_extract_files_from_dash");
    let input = base.join("in");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("keep_a.txt"), "keep-a").unwrap();

    let archive_path = base.join("archive.pna");
    let mut create_cmd = cargo_bin_cmd!("pna");
    create_cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--create",
        "--unstable",
        "--overwrite",
        "-f",
        archive_path.to_str().unwrap(),
        "-C",
        input.to_str().unwrap(),
        "keep_a.txt",
    ]);
    create_cmd.assert().success();

    let output_dir = base.join("out");
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--extract",
            "--unstable",
            "--files-from",
            "-",
            "--out-dir",
            output_dir.to_str().unwrap(),
            "--overwrite",
            "-f",
            archive_path.to_str().unwrap(),
        ])
        .write_stdin("keep_a.txt\n");
    extract_cmd.assert().failure();
}

/// Precondition: Appending with the archive read from stdin.
/// Action: Run `pna compat bsdtar --append -f - -T -` with a file list on stdin.
/// Expectation: The command fails with a stdin conflict error instead of consuming stdin twice.
#[test]
fn bsdtar_append_files_from_stdin_and_archive_stdin_fails() {
    setup();

    let mut append_cmd = cargo_bin_cmd!("pna");
    append_cmd
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--append",
            "--unstable",
            "-f",
            "-",
            "-T",
            "-",
        ])
        .write_stdin("keep_a.txt\n");
    append_cmd
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be combined"));
}

/// Precondition: An archive exists.
/// Action: Run `pna compat bsdtar --append` without input paths or `-T`.
/// Expectation: The command fails instead of appending nothing.
#[test]
fn bsdtar_append_without_inputs_fails() {
    setup();

    let base = Path::new("bsdtar_append_without_inputs");
    let input = base.join("in");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("keep_a.txt"), "keep-a").unwrap();

    let archive_path = base.join("archive.pna");
    let mut create_cmd = cargo_bin_cmd!("pna");
    create_cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--create",
        "--unstable",
        "--overwrite",
        "-f",
        archive_path.to_str().unwrap(),
        "-C",
        input.to_str().unwrap(),
        "keep_a.txt",
    ]);
    create_cmd.assert().success();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--append",
            "--unstable",
            "-f",
            archive_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("append mode requires"));
}

/// Precondition: An archive exists.
/// Action: Run `pna compat bsdtar --update` without input paths or `-T`.
/// Expectation: The command fails instead of updating nothing.
#[test]
fn bsdtar_update_without_inputs_fails() {
    setup();

    let base = Path::new("bsdtar_update_without_inputs");
    let input = base.join("in");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("keep_a.txt"), "keep-a").unwrap();

    let archive_path = base.join("archive.pna");
    let mut create_cmd = cargo_bin_cmd!("pna");
    create_cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--create",
        "--unstable",
        "--overwrite",
        "-f",
        archive_path.to_str().unwrap(),
        "-C",
        input.to_str().unwrap(),
        "keep_a.txt",
    ]);
    create_cmd.assert().success();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--update",
            "--unstable",
            "-f",
            archive_path.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("update mode requires"));
}

/// Precondition: A manifest file is empty.
/// Action: Create the archive via `pna compat bsdtar --create -T <manifest>`.
/// Expectation: The command succeeds and the archive contains no entries.
#[test]
fn bsdtar_create_with_empty_files_from_succeeds() {
    setup();

    let base = Path::new("bsdtar_create_empty_files_from");
    let manifest_path = base.join("empty.txt");
    fs::create_dir_all(base).unwrap();
    fs::write(&manifest_path, "").unwrap();

    let archive_path = base.join("archive.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            archive_path.to_str().unwrap(),
            "-T",
            manifest_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let mut names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        names.push(entry.header().path().to_string());
    })
    .unwrap();
    assert!(names.is_empty(), "expected no entries, got {names:?}");
}

/// Precondition: An archive exists and a manifest file is empty.
/// Action: Run `pna compat bsdtar --append -T <manifest>`.
/// Expectation: The command succeeds and the archive is unchanged.
#[test]
fn bsdtar_append_with_empty_files_from_succeeds() {
    setup();

    let base = Path::new("bsdtar_append_empty_files_from");
    let input = base.join("in");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("keep_a.txt"), "keep-a").unwrap();

    let manifest_path = base.join("empty.txt");
    fs::write(&manifest_path, "").unwrap();

    let archive_path = base.join("archive.pna");
    let mut create_cmd = cargo_bin_cmd!("pna");
    create_cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--create",
        "--unstable",
        "--overwrite",
        "-f",
        archive_path.to_str().unwrap(),
        "-C",
        input.to_str().unwrap(),
        "keep_a.txt",
    ]);
    create_cmd.assert().success();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--append",
            "--unstable",
            "-f",
            archive_path.to_str().unwrap(),
            "-T",
            manifest_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let mut names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        names.push(entry.header().path().to_string());
    })
    .unwrap();
    names.sort();
    assert_eq!(names, ["keep_a.txt"]);
}

/// Precondition: An archive exists and a manifest file is empty.
/// Action: Run `pna compat bsdtar --update -T <manifest>`.
/// Expectation: The command succeeds and the archive is unchanged.
#[test]
fn bsdtar_update_with_empty_files_from_succeeds() {
    setup();

    let base = Path::new("bsdtar_update_empty_files_from");
    let input = base.join("in");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("keep_a.txt"), "keep-a").unwrap();

    let manifest_path = base.join("empty.txt");
    fs::write(&manifest_path, "").unwrap();

    let archive_path = base.join("archive.pna");
    let mut create_cmd = cargo_bin_cmd!("pna");
    create_cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--create",
        "--unstable",
        "--overwrite",
        "-f",
        archive_path.to_str().unwrap(),
        "-C",
        input.to_str().unwrap(),
        "keep_a.txt",
    ]);
    create_cmd.assert().success();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--update",
            "--unstable",
            "-f",
            archive_path.to_str().unwrap(),
            "-T",
            manifest_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let mut names = Vec::new();
    archive::for_each_entry(&archive_path, |entry| {
        names.push(entry.header().path().to_string());
    })
    .unwrap();
    names.sort();
    assert_eq!(names, ["keep_a.txt"]);
}
