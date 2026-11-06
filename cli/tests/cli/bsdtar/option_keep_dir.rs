use crate::utils::{archive, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use clap::Parser;
use portable_network_archive::cli;
use std::{fs, path::Path};

fn sorted_entry_names(path: &Path) -> Vec<String> {
    let mut names = archive::get_archive_entry_names(path)
        .into_iter()
        .map(|name| name.trim_end_matches('/').to_owned())
        .collect::<Vec<_>>();
    names.sort();
    names
}

/// Precondition: A directory contains a regular file.
/// Action: Create bsdtar-compatible archives with default, positive, and negative directory flags.
/// Expectation: Directory entries follow the flag while file entries are always retained.
#[test]
fn bsdtar_create_keep_dir_flags_control_directory_entries() {
    setup();
    let base = Path::new("bsdtar_keep_dir_create");
    fs::create_dir_all(base.join("source")).unwrap();
    fs::write(base.join("source/file.txt"), "content").unwrap();

    for (archive_name, flags, expected) in [
        ("default.pna", &[][..], &["source", "source/file.txt"][..]),
        ("positive.pna", &["--keep-dir"][..], &["source", "source/file.txt"][..]),
        ("negative.pna", &["--no-keep-dir"][..], &["source/file.txt"][..]),
    ] {
        let output = base.join(archive_name);
        cargo_bin_cmd!("pna")
            .args([
                "--quiet",
                "compat",
                "bsdtar",
                "--create",
                "--overwrite",
                "-f",
                output.to_str().unwrap(),
                "-C",
                base.to_str().unwrap(),
            ])
            .args(flags)
            .arg("source")
            .assert()
            .success();

        assert_eq!(sorted_entry_names(&output), expected, "{archive_name}");
    }
}

/// Precondition: An archive contains a file and a directory source has not yet been archived.
/// Action: Append or update using default and negative directory flags.
/// Expectation: The negative flag excludes directories without dropping regular files.
#[test]
fn bsdtar_append_update_keep_dir_flags_control_directory_entries() {
    setup();
    let base = Path::new("bsdtar_keep_dir_append_update");
    fs::create_dir_all(base.join("source")).unwrap();
    fs::write(base.join("source/file.txt"), "content").unwrap();

    for (archive_name, action, flags, expected) in [
        (
            "append_default.pna",
            "--append",
            &[][..],
            &["initial.txt", "source", "source/file.txt"][..],
        ),
        (
            "append_negative.pna",
            "--append",
            &["--no-keep-dir"][..],
            &["initial.txt", "source/file.txt"][..],
        ),
        (
            "update_default.pna",
            "--update",
            &[][..],
            &["initial.txt", "source", "source/file.txt"][..],
        ),
        (
            "update_negative.pna",
            "--update",
            &["--no-keep-dir"][..],
            &["initial.txt", "source/file.txt"][..],
        ),
    ] {
        let output = base.join(archive_name);
        archive::create_test_archive(&output, &[("initial.txt", "initial")]);
        cargo_bin_cmd!("pna")
            .args([
                "--quiet",
                "compat",
                "bsdtar",
                action,
                "-f",
                output.to_str().unwrap(),
                "-C",
                base.to_str().unwrap(),
            ])
            .args(flags)
            .arg("source")
            .assert()
            .success();

        assert_eq!(sorted_entry_names(&output), expected, "{archive_name}");
    }
}

/// Precondition: bsdtar exposes positive and negative directory flags.
/// Action: Specify both flags in either order.
/// Expectation: The parser rejects conflicting directives.
#[test]
fn bsdtar_keep_dir_flags_conflict_in_either_order() {
    for flags in [
        ["--keep-dir", "--no-keep-dir"],
        ["--no-keep-dir", "--keep-dir"],
    ] {
        let mut args = vec!["pna", "compat", "bsdtar", "--create", "-f", "unused.pna"];
        args.extend(flags);
        let err = cli::Cli::try_parse_from(&args).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
    }
}
