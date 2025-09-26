use crate::utils::{self, diff::diff, setup, EmbedExt, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn extract_with_exclude_vcs() {
    setup();
    TestResources::extract_in("raw/", "extract_with_exclude_vcs/in/").unwrap();

    // Create VCS files and directories
    let vcs_files = [
        "extract_with_exclude_vcs/in/raw/.git/HEAD",
        "extract_with_exclude_vcs/in/raw/.git/config",
        "extract_with_exclude_vcs/in/raw/.gitignore",
        "extract_with_exclude_vcs/in/raw/.svn/entries",
        "extract_with_exclude_vcs/in/raw/.hg/hgrc",
        "extract_with_exclude_vcs/in/raw/.hgignore",
        "extract_with_exclude_vcs/in/raw/.bzr/branch-format",
        "extract_with_exclude_vcs/in/raw/.bzrignore",
        "extract_with_exclude_vcs/in/raw/CVS/Root",
        "extract_with_exclude_vcs/in/raw/.gitmodules",
        "extract_with_exclude_vcs/in/raw/.gitattributes",
    ];

    for file in vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }

    // Create some regular files that should be included
    let regular_files = [
        "extract_with_exclude_vcs/in/raw/regular.txt",
        "extract_with_exclude_vcs/in/raw/data.csv",
        "extract_with_exclude_vcs/in/raw/document.pdf",
    ];

    for file in regular_files {
        fs::write(file, "regular file content").unwrap();
    }

    // Create archive with all files
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_with_exclude_vcs/extract_with_exclude_vcs.pna",
        "--overwrite",
        "extract_with_exclude_vcs/in/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract with exclude-vcs option
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_with_exclude_vcs/extract_with_exclude_vcs.pna",
        "--overwrite",
        "--out-dir",
        "extract_with_exclude_vcs/out/",
        "--strip-components",
        "2",
        "--exclude-vcs",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Remove VCS files that are expected to be excluded from input for comparison
    for file in vcs_files {
        utils::remove_with_empty_parents(file).unwrap();
    }

    diff(
        "extract_with_exclude_vcs/in/",
        "extract_with_exclude_vcs/out/",
    )
    .unwrap();
}

#[test]
fn extract_without_exclude_vcs() {
    setup();
    TestResources::extract_in("raw/", "extract_without_exclude_vcs/in/").unwrap();

    // Create VCS files and directories
    let vcs_files = [
        "extract_without_exclude_vcs/in/raw/.git/HEAD",
        "extract_without_exclude_vcs/in/raw/.git/config",
        "extract_without_exclude_vcs/in/raw/.gitignore",
        "extract_without_exclude_vcs/in/raw/.svn/entries",
        "extract_without_exclude_vcs/in/raw/.hg/hgrc",
        "extract_without_exclude_vcs/in/raw/.hgignore",
        "extract_without_exclude_vcs/in/raw/.bzr/branch-format",
        "extract_without_exclude_vcs/in/raw/.bzrignore",
        "extract_without_exclude_vcs/in/raw/CVS/Root",
        "extract_without_exclude_vcs/in/raw/.gitmodules",
        "extract_without_exclude_vcs/in/raw/.gitattributes",
    ];

    for file in vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }

    // Create some regular files
    let regular_files = [
        "extract_without_exclude_vcs/in/raw/regular.txt",
        "extract_without_exclude_vcs/in/raw/data.csv",
        "extract_without_exclude_vcs/in/raw/document.pdf",
    ];

    for file in regular_files {
        fs::write(file, "regular file content").unwrap();
    }

    // Create archive with all files
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "extract_without_exclude_vcs/extract_without_exclude_vcs.pna",
        "--overwrite",
        "extract_without_exclude_vcs/in/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Extract without exclude-vcs option
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "extract_without_exclude_vcs/extract_without_exclude_vcs.pna",
        "--overwrite",
        "--out-dir",
        "extract_without_exclude_vcs/out/",
        "--strip-components",
        "2",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // VCS files should be included when --exclude-vcs is not used
    diff(
        "extract_without_exclude_vcs/in/",
        "extract_without_exclude_vcs/out/",
    )
    .unwrap();
}
