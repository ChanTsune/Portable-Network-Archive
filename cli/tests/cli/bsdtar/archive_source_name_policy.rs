use crate::utils::{archive::for_each_entry, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::{Path, PathBuf};

/// Stored names, which the shared `get_archive_entry_names` does not report: it
/// returns `header().path()`, and the `./` prefix it sanitizes away is the
/// difference under test.
fn stored_names(archive: impl AsRef<Path>) -> Vec<String> {
    let mut names = Vec::new();
    for_each_entry(archive, |entry| names.push(entry.name().to_string())).unwrap();
    names.sort();
    names
}

#[test]
fn bsdtar_archive_source_keeps_curdir_prefix() {
    setup();

    let base = PathBuf::from("bsdtar_archive_source_keeps_curdir_prefix");
    fs::create_dir_all(base.join("d")).unwrap();
    fs::write(base.join("d/f"), "content").unwrap();

    let source = base.join("source.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--create",
            "--overwrite",
            "-f",
            source.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "./d/f",
        ])
        .assert()
        .success();
    assert_eq!(stored_names(&source), ["./d/f"]);

    let copied = base.join("copied.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--create",
            "--unstable",
            "--overwrite",
            "-f",
            copied.to_str().unwrap(),
            "-C",
            base.to_str().unwrap(),
            "@source.pna",
        ])
        .assert()
        .success();
    assert_eq!(stored_names(&copied), ["./d/f"]);
}

#[cfg(unix)]
#[test]
fn bsdtar_archive_source_keeps_absolute_path_with_absolute_paths() {
    setup();

    let base = PathBuf::from("bsdtar_archive_source_keeps_absolute_path");
    fs::create_dir_all(base.join("d")).unwrap();
    fs::write(base.join("d/f"), "content").unwrap();
    let absolute = base
        .join("d/f")
        .canonicalize()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    let source = base.join("source.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--create",
            "--unstable",
            "--absolute-paths",
            "--overwrite",
            "-f",
            source.to_str().unwrap(),
            &absolute,
        ])
        .assert()
        .success();
    assert_eq!(stored_names(&source), [absolute.as_str()]);

    let copied = base.join("copied.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--create",
            "--unstable",
            "--absolute-paths",
            "--overwrite",
            "-f",
            copied.to_str().unwrap(),
            &format!("@{}", source.display()),
        ])
        .assert()
        .success();
    assert_eq!(stored_names(&copied), [absolute.as_str()]);
}
