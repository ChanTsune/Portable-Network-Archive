use crate::utils::{
    EmbedExt, TestResources,
    archive::{create_test_archive, get_archive_entry_names},
    diff::assert_dirs_equal,
    setup,
};
use assert_cmd::cargo::cargo_bin_cmd;
use std::collections::HashSet;
use std::fs;

/// Precondition: Source tree is available for bsdtar archive creation.
/// Action: Create an archive via `pna compat bsdtar -c -C <input> .`,
///         then extract it via stdin with `pna compat bsdtar -x -C . --out-dir <out>`.
/// Expectation: Directory changes are applied before resolving create inputs and extract output.
#[test]
fn bsdtar_create_extract_with_cd() {
    setup();
    TestResources::extract_in("raw/", "bsdtar_with_cd/in/").unwrap();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "-c",
        "-C",
        "bsdtar_with_cd/in/",
        ".",
    ]);
    let assert = cmd.assert().success();

    let mut cmd = cargo_bin_cmd!("pna");
    cmd.write_stdin(assert.get_output().stdout.as_slice());
    cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "-x",
        "--overwrite",
        "-C",
        ".",
        "--out-dir",
        "bsdtar_with_cd/out/",
    ]);
    cmd.assert().success();

    assert_dirs_equal("bsdtar_with_cd/in/", "bsdtar_with_cd/out/");
}

/// Precondition: An archive and a filesystem file are available in separate directories.
/// Action: Include the archive, then change directories before adding the filesystem file.
/// Expectation: Both inputs are resolved from the directory active at their position.
#[test]
fn bsdtar_create_archive_inclusion_before_cd() {
    setup();

    let base = fs::canonicalize(".")
        .unwrap()
        .join("stdio_cd_archive_before_cd");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    let source_archive = base.join("source.pna");
    create_test_archive(&source_archive, &[("a.txt", "content a")]);

    let sub_dir = base.join("sub");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("b.txt"), "content b").unwrap();

    let output_archive = base.join("result.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@source.pna",
            "-C",
            sub_dir.to_str().unwrap(),
            "b.txt",
        ])
        .assert()
        .success();

    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("a.txt"), "Missing a.txt from @source");
    assert!(
        entry_names.contains("b.txt"),
        "Missing b.txt from sub directory"
    );
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: Files are available in two separate directories.
/// Action: Add one file from each directory using successive directory changes.
/// Expectation: Both files are archived under their input path names.
#[test]
fn bsdtar_create_multiple_cd_absolute_paths() {
    setup();

    let base = fs::canonicalize(".")
        .unwrap()
        .join("stdio_cd_multiple_absolute");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    let d1 = base.join("d1");
    fs::create_dir_all(&d1).unwrap();
    fs::write(d1.join("f1.txt"), "content f1").unwrap();

    let d2 = base.join("d2");
    fs::create_dir_all(&d2).unwrap();
    fs::write(d2.join("f2.txt"), "content f2").unwrap();

    let output_archive = base.join("result.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            d1.to_str().unwrap(),
            "f1.txt",
            "-C",
            d2.to_str().unwrap(),
            "f2.txt",
        ])
        .assert()
        .success();

    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("f1.txt"), "Missing f1.txt");
    assert!(entry_names.contains("f2.txt"), "Missing f2.txt");
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: A nested directory tree includes a file with an option-like name.
/// Action: Change directories between inputs and include the option-like name literally.
/// Expectation: Each input is resolved from the directory active at its position.
#[test]
fn bsdtar_create_cd_does_not_affect_prior_args() {
    setup();

    let base = fs::canonicalize(".").unwrap().join("stdio_cd_ordering");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    let alpha = base.join("alpha");
    fs::create_dir_all(&alpha).unwrap();
    fs::write(alpha.join("first.txt"), "first").unwrap();

    let beta = alpha.join("beta");
    fs::create_dir_all(&beta).unwrap();
    fs::write(beta.join("second.txt"), "second").unwrap();

    fs::write(beta.join("-C"), "literal").unwrap();
    let output_archive = base.join("result.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            alpha.to_str().unwrap(),
            "first.txt",
            "--directory=beta",
            "second.txt",
            "--",
            "-C",
        ])
        .assert()
        .success();

    let entry_names = get_archive_entry_names(&output_archive);
    assert_eq!(entry_names, ["first.txt", "second.txt", "-C"]);
}

/// Precondition: An archive and a filesystem file are available in separate directories.
/// Action: Change directories before including each source.
/// Expectation: Each source is resolved from the directory active at its position.
#[test]
fn bsdtar_create_cd_affects_archive_inclusion() {
    setup();

    let base = fs::canonicalize(".")
        .unwrap()
        .join("stdio_cd_archive_resolution");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    let src_dir = base.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let source_archive = src_dir.join("source.pna");
    create_test_archive(&source_archive, &[("archived.txt", "from archive")]);

    let dir = base.join("dir");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("extra.txt"), "extra content").unwrap();

    let output_archive = base.join("result.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            src_dir.to_str().unwrap(),
            "@source.pna",
            "-C",
            dir.to_str().unwrap(),
            "extra.txt",
        ])
        .assert()
        .success();

    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(
        entry_names.contains("archived.txt"),
        "Missing archived.txt from @source"
    );
    assert!(
        entry_names.contains("extra.txt"),
        "Missing extra.txt from dir/"
    );
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: An archive and a symlink to an output directory are available.
/// Action: Extract with the symlink specified as the change directory.
/// Expectation: Extracted entries appear in the directory target.
#[test]
fn bsdtar_extract_with_cd() {
    setup();

    let base = fs::canonicalize(".").unwrap().join("stdio_extract_with_cd");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    let archive = base.join("test.pna");
    create_test_archive(&archive, &[("a.txt", "alpha"), ("b.txt", "beta")]);

    let target = base.join("out");
    fs::create_dir_all(&target).unwrap();

    let alias = base.join("alias");
    pna::fs::symlink(&target, &alias).unwrap();
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--extract",
            "--unstable",
            "-f",
            archive.to_str().unwrap(),
            "-C",
            alias.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert_eq!(fs::read_to_string(target.join("a.txt")).unwrap(), "alpha");
    assert_eq!(fs::read_to_string(target.join("b.txt")).unwrap(), "beta");
}

/// Precondition: An archive and a source file are available in separate directories.
/// Action: Update the archive after changing to the source directory.
/// Expectation: The archive retains its original entry and includes the selected source.
#[test]
fn bsdtar_update_with_cd() {
    setup();

    let base = fs::canonicalize(".").unwrap().join("stdio_update_with_cd");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    let archive = base.join("test.pna");
    create_test_archive(&archive, &[("a.txt", "content a")]);

    let sub_dir = base.join("sub");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("b.txt"), "content b").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--update",
            "--unstable",
            "-f",
            archive.to_str().unwrap(),
            "-C",
            sub_dir.to_str().unwrap(),
            "b.txt",
        ])
        .assert()
        .success();

    let entry_names: HashSet<String> = get_archive_entry_names(&archive).into_iter().collect();
    assert!(entry_names.contains("a.txt"), "Missing a.txt");
    assert!(entry_names.contains("b.txt"), "Missing b.txt");
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: An archive and a source file are available in separate directories.
/// Action: Append to the archive after changing to the source directory.
/// Expectation: The archive retains its original entry and includes the selected source.
#[test]
fn bsdtar_append_with_cd() {
    setup();

    let base = fs::canonicalize(".").unwrap().join("stdio_append_with_cd");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    let archive = base.join("test.pna");
    create_test_archive(&archive, &[("a.txt", "content a")]);

    let sub_dir = base.join("sub");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("b.txt"), "content b").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--append",
            "--unstable",
            "-f",
            archive.to_str().unwrap(),
            "-C",
            sub_dir.to_str().unwrap(),
            "b.txt",
        ])
        .assert()
        .success();

    let entry_names: HashSet<String> = get_archive_entry_names(&archive).into_iter().collect();
    assert!(entry_names.contains("a.txt"), "Missing a.txt");
    assert!(entry_names.contains("b.txt"), "Missing b.txt");
    assert_eq!(entry_names.len(), 2);
}
