use crate::utils::{archive::for_each_entry, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{Archive, EntryBuilder, WriteOptions};
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::Path;

fn create_test_archive(path: &Path, entries: &[(&str, &str)]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();
    for (name, contents) in entries {
        writer
            .add_entry({
                let mut builder =
                    EntryBuilder::new_file((*name).into(), WriteOptions::builder().build())
                        .unwrap();
                builder.write_all(contents.as_bytes()).unwrap();
                builder.build().unwrap()
            })
            .unwrap();
    }
    writer.finalize().unwrap();
}

fn get_archive_entry_names(path: &Path) -> Vec<String> {
    let mut names = Vec::new();
    for_each_entry(path, |entry| {
        names.push(entry.header().path().to_string());
    })
    .unwrap();
    names
}

/// Precondition: Archive source.pna contains file a.txt. Directory sub/ contains b.txt.
/// Action: Create with `@source.pna -C sub b.txt` -- archive inclusion before -C.
/// Expectation: Result contains both a.txt (from @source) and b.txt (from sub/).
///   @source.pna is resolved from original cwd, not from sub/.
#[test]
fn stdio_create_archive_inclusion_before_cd() {
    setup();

    let base = fs::canonicalize(".")
        .unwrap()
        .join("stdio_cd_archive_before_cd");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    // Create source archive with a.txt
    let source_archive = base.join("source.pna");
    create_test_archive(&source_archive, &[("a.txt", "content a")]);

    // Create sub directory with b.txt
    let sub_dir = base.join("sub");
    fs::create_dir_all(&sub_dir).unwrap();
    fs::write(sub_dir.join("b.txt"), "content b").unwrap();

    // Create result archive: @source.pna is before -C, so it resolves from base (cwd).
    // -C sub applies only to b.txt.
    let output_archive = base.join("result.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
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

    // Verify the output archive contains both entries
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

/// Precondition: Directories d1/ and d2/ each contain a file.
/// Action: Create with `-C d1 f1.txt -C d2 f2.txt` using absolute -C paths.
/// Expectation: Both files archived with base names only (no directory prefix).
#[test]
fn stdio_create_multiple_cd_absolute_paths() {
    setup();

    let base = fs::canonicalize(".")
        .unwrap()
        .join("stdio_cd_multiple_absolute");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    // Create d1 with f1.txt
    let d1 = base.join("d1");
    fs::create_dir_all(&d1).unwrap();
    fs::write(d1.join("f1.txt"), "content f1").unwrap();

    // Create d2 with f2.txt
    let d2 = base.join("d2");
    fs::create_dir_all(&d2).unwrap();
    fs::write(d2.join("f2.txt"), "content f2").unwrap();

    let output_archive = base.join("result.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
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

    // Verify both files are archived with base names
    let entry_names: HashSet<String> = get_archive_entry_names(&output_archive)
        .into_iter()
        .collect();
    assert!(entry_names.contains("f1.txt"), "Missing f1.txt");
    assert!(entry_names.contains("f2.txt"), "Missing f2.txt");
    assert_eq!(entry_names.len(), 2);
}

/// Precondition: A directory tree with nested files exists.
/// Action: Create with `-C` changing into different directories for each file group.
/// Expectation: Each file is archived relative to its -C directory.
#[test]
fn stdio_create_cd_does_not_affect_prior_args() {
    setup();

    let base = fs::canonicalize(".").unwrap().join("stdio_cd_ordering");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    // Create directories
    let alpha = base.join("alpha");
    fs::create_dir_all(&alpha).unwrap();
    fs::write(alpha.join("first.txt"), "first").unwrap();

    let beta = base.join("beta");
    fs::create_dir_all(&beta).unwrap();
    fs::write(beta.join("second.txt"), "second").unwrap();

    // Archive first.txt from alpha, then second.txt from beta
    let output_archive = base.join("result.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            output_archive.to_str().unwrap(),
            "-C",
            alpha.to_str().unwrap(),
            "first.txt",
            "-C",
            beta.to_str().unwrap(),
            "second.txt",
        ])
        .assert()
        .success();

    // Verify entry order matches argument order
    let entry_names = get_archive_entry_names(&output_archive);
    assert_eq!(entry_names.len(), 2, "Expected exactly 2 entries");
    assert_eq!(entry_names[0], "first.txt");
    assert_eq!(entry_names[1], "second.txt");
}

/// Precondition: Archive source.pna exists in directory src/. File extra.txt exists in dir/.
/// Action: Create with `-C src @source.pna -C dir extra.txt`.
/// Expectation: @source.pna resolves from src/ (the active -C), extra.txt resolves from dir/.
#[test]
fn stdio_create_cd_affects_archive_inclusion() {
    setup();

    let base = fs::canonicalize(".")
        .unwrap()
        .join("stdio_cd_archive_resolution");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    // Create source archive inside src/ subdirectory
    let src_dir = base.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let source_archive = src_dir.join("source.pna");
    create_test_archive(&source_archive, &[("archived.txt", "from archive")]);

    // Create extra file in dir/ subdirectory
    let dir = base.join("dir");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("extra.txt"), "extra content").unwrap();

    // -C src makes @source.pna resolve from src/
    // -C dir makes extra.txt resolve from dir/
    let output_archive = base.join("result.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "stdio",
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

    // Verify both entries present
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
