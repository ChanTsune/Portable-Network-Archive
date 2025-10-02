use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::collections::HashSet;
use std::fs;

/// Precondition: The source tree contains VCS metadata files.
/// Action: Run `pna create` to build an archive, then delete entries from the archive by
///         `pna experimental delete` with `--exclude-vcs`.
/// Expectation: All non-VCS entries matching the pattern are removed, while VCS metadata entries
///         remain in the archive.
#[test]
fn delete_with_exclude_vcs() {
    setup();
    let vcs_files = [
        "delete_with_exclude_vcs/in/raw/.git/HEAD",
        "delete_with_exclude_vcs/in/raw/.git/config",
        "delete_with_exclude_vcs/in/raw/.gitignore",
        "delete_with_exclude_vcs/in/raw/.svn/entries",
        "delete_with_exclude_vcs/in/raw/.hg/hgrc",
        "delete_with_exclude_vcs/in/raw/.hgignore",
        "delete_with_exclude_vcs/in/raw/.bzr/branch-format",
        "delete_with_exclude_vcs/in/raw/.bzrignore",
        "delete_with_exclude_vcs/in/raw/CVS/Root",
        "delete_with_exclude_vcs/in/raw/.gitmodules",
        "delete_with_exclude_vcs/in/raw/.gitattributes",
    ];
    for file in vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_with_exclude_vcs/delete_with_exclude_vcs.pna",
        "--overwrite",
        "delete_with_exclude_vcs/in/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_with_exclude_vcs/delete_with_exclude_vcs.pna",
        "**/.git/**",
        "--unstable",
        "--exclude-vcs",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "delete_with_exclude_vcs/delete_with_exclude_vcs.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    for required in [
        "delete_with_exclude_vcs/in/raw/.git/HEAD",
        "delete_with_exclude_vcs/in/raw/.git/config",
        "delete_with_exclude_vcs/in/raw/.gitignore",
        "delete_with_exclude_vcs/in/raw/.svn/entries",
        "delete_with_exclude_vcs/in/raw/.hg/hgrc",
        "delete_with_exclude_vcs/in/raw/.hgignore",
        "delete_with_exclude_vcs/in/raw/.bzr/branch-format",
        "delete_with_exclude_vcs/in/raw/.bzrignore",
        "delete_with_exclude_vcs/in/raw/CVS/Root",
        "delete_with_exclude_vcs/in/raw/.gitmodules",
        "delete_with_exclude_vcs/in/raw/.gitattributes",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }

    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

/// Precondition: The source tree contains VCS metadata files.
/// Action: Run `pna create` to build an archive, then delete entries from the archive by
///         `pna experimental delete` without `--exclude-vcs`.
/// Expectation: All matching entries, including VCS metadata, are removed.
#[test]
fn delete_without_exclude_vcs() {
    setup();
    let vcs_files = [
        "delete_without_exclude_vcs/in/raw/.git/HEAD",
        "delete_without_exclude_vcs/in/raw/.git/config",
        "delete_without_exclude_vcs/in/raw/.gitignore",
        "delete_without_exclude_vcs/in/raw/.svn/entries",
        "delete_without_exclude_vcs/in/raw/.hg/hgrc",
        "delete_without_exclude_vcs/in/raw/.hgignore",
        "delete_without_exclude_vcs/in/raw/.bzr/branch-format",
        "delete_without_exclude_vcs/in/raw/.bzrignore",
        "delete_without_exclude_vcs/in/raw/CVS/Root",
        "delete_without_exclude_vcs/in/raw/.gitmodules",
        "delete_without_exclude_vcs/in/raw/.gitattributes",
    ];
    for file in vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "delete_without_exclude_vcs/delete_without_exclude_vcs.pna",
        "--overwrite",
        "delete_without_exclude_vcs/in/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "-f",
        "delete_without_exclude_vcs/delete_without_exclude_vcs.pna",
        "**/.git/**",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "delete_without_exclude_vcs/delete_without_exclude_vcs.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    for required in [
        "delete_without_exclude_vcs/in/raw/.gitignore",
        "delete_without_exclude_vcs/in/raw/.svn/entries",
        "delete_without_exclude_vcs/in/raw/.hg/hgrc",
        "delete_without_exclude_vcs/in/raw/.hgignore",
        "delete_without_exclude_vcs/in/raw/.bzr/branch-format",
        "delete_without_exclude_vcs/in/raw/.bzrignore",
        "delete_without_exclude_vcs/in/raw/CVS/Root",
        "delete_without_exclude_vcs/in/raw/.gitmodules",
        "delete_without_exclude_vcs/in/raw/.gitattributes",
    ] {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }
    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
