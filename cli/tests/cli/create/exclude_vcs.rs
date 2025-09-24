use crate::utils::{self, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn create_with_exclude_vcs() {
    setup();
    TestResources::extract_in("raw/", "create_with_exclude_vcs/in/").unwrap();

    // Create VCS files and directories
    let vcs_files = [
        "create_with_exclude_vcs/in/raw/.git/HEAD",
        "create_with_exclude_vcs/in/raw/.git/config",
        "create_with_exclude_vcs/in/raw/.gitignore",
        "create_with_exclude_vcs/in/raw/.svn/entries",
        "create_with_exclude_vcs/in/raw/.hg/hgrc",
        "create_with_exclude_vcs/in/raw/.hgignore",
        "create_with_exclude_vcs/in/raw/.bzr/branch-format",
        "create_with_exclude_vcs/in/raw/.bzrignore",
        "create_with_exclude_vcs/in/raw/CVS/Root",
        "create_with_exclude_vcs/in/raw/.gitmodules",
        "create_with_exclude_vcs/in/raw/.gitattributes",
    ];

    for file in vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }

    // Create some regular files that should be included
    let regular_files = [
        "create_with_exclude_vcs/in/raw/regular.txt",
        "create_with_exclude_vcs/in/raw/data.csv",
        "create_with_exclude_vcs/in/raw/document.pdf",
    ];

    for file in regular_files {
        fs::write(file, "regular file content").unwrap();
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_with_exclude_vcs/create_with_exclude_vcs.pna",
        "--overwrite",
        "create_with_exclude_vcs/in/",
        "--exclude-vcs",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::try_exists("create_with_exclude_vcs/create_with_exclude_vcs.pna").unwrap());

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "create_with_exclude_vcs/create_with_exclude_vcs.pna",
        "--overwrite",
        "--out-dir",
        "create_with_exclude_vcs/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Remove VCS files that are expected to be excluded from input for comparison
    for file in vcs_files {
        utils::remove_with_empty_parents(file).unwrap();
    }

    diff(
        "create_with_exclude_vcs/in/",
        "create_with_exclude_vcs/out/",
    )
    .unwrap();
}

#[test]
fn create_without_exclude_vcs() {
    setup();
    TestResources::extract_in("raw/", "create_without_exclude_vcs/in/").unwrap();

    // Create VCS files and directories
    let vcs_files = [
        "create_without_exclude_vcs/in/raw/.git/HEAD",
        "create_without_exclude_vcs/in/raw/.git/config",
        "create_without_exclude_vcs/in/raw/.gitignore",
        "create_without_exclude_vcs/in/raw/.svn/entries",
        "create_without_exclude_vcs/in/raw/.hg/hgrc",
        "create_without_exclude_vcs/in/raw/.hgignore",
        "create_without_exclude_vcs/in/raw/.bzr/branch-format",
        "create_without_exclude_vcs/in/raw/.bzrignore",
        "create_without_exclude_vcs/in/raw/CVS/Root",
        "create_without_exclude_vcs/in/raw/.gitmodules",
        "create_without_exclude_vcs/in/raw/.gitattributes",
    ];

    for file in vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }

    // Create some regular files
    let regular_files = [
        "create_without_exclude_vcs/in/raw/regular.txt",
        "create_without_exclude_vcs/in/raw/data.csv",
        "create_without_exclude_vcs/in/raw/document.pdf",
    ];

    for file in regular_files {
        fs::write(file, "regular file content").unwrap();
    }

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "create_without_exclude_vcs/create_without_exclude_vcs.pna",
        "--overwrite",
        "create_without_exclude_vcs/in/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    assert!(fs::try_exists("create_without_exclude_vcs/create_without_exclude_vcs.pna").unwrap());

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "create_without_exclude_vcs/create_without_exclude_vcs.pna",
        "--overwrite",
        "--out-dir",
        "create_without_exclude_vcs/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // VCS files should be included when --exclude-vcs is not used
    diff(
        "create_without_exclude_vcs/in/",
        "create_without_exclude_vcs/out/",
    )
    .unwrap();
}
