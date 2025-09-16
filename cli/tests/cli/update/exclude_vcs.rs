use crate::utils::{self, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn update_with_exclude_vcs() {
    setup();
    TestResources::extract_in("raw/", "update_with_exclude_vcs/in/").unwrap();
    let vcs_files = [
        "update_with_exclude_vcs/in/raw/.git/HEAD",
        "update_with_exclude_vcs/in/raw/.git/config",
        "update_with_exclude_vcs/in/raw/.gitignore",
        "update_with_exclude_vcs/in/raw/.svn/entries",
        "update_with_exclude_vcs/in/raw/.hg/hgrc",
        "update_with_exclude_vcs/in/raw/.hgignore",
        "update_with_exclude_vcs/in/raw/.bzr/branch-format",
        "update_with_exclude_vcs/in/raw/.bzrignore",
        "update_with_exclude_vcs/in/raw/CVS/Root",
        "update_with_exclude_vcs/in/raw/.gitmodules",
        "update_with_exclude_vcs/in/raw/.gitattributes",
    ];
    for file in vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }
    let regular_files = [
        "update_with_exclude_vcs/in/raw/regular.txt",
        "update_with_exclude_vcs/in/raw/data.csv",
        "update_with_exclude_vcs/in/raw/document.pdf",
    ];
    for file in regular_files {
        fs::write(file, "regular file content").unwrap();
    }
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_with_exclude_vcs/update_with_exclude_vcs.pna",
        "--overwrite",
        "update_with_exclude_vcs/in/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    // update with exclude-vcs
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_with_exclude_vcs/update_with_exclude_vcs.pna",
        "update_with_exclude_vcs/in/raw/",
        "--unstable",
        "--exclude-vcs",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_with_exclude_vcs/update_with_exclude_vcs.pna",
        "--overwrite",
        "--out-dir",
        "update_with_exclude_vcs/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    for file in vcs_files {
        utils::remove_with_empty_parents(file).unwrap();
    }
    diff(
        "update_with_exclude_vcs/in/",
        "update_with_exclude_vcs/out/",
    )
    .unwrap();
}

#[test]
fn update_without_exclude_vcs() {
    setup();
    TestResources::extract_in("raw/", "update_without_exclude_vcs/in/").unwrap();
    let vcs_files = [
        "update_without_exclude_vcs/in/raw/.git/HEAD",
        "update_without_exclude_vcs/in/raw/.git/config",
        "update_without_exclude_vcs/in/raw/.gitignore",
        "update_without_exclude_vcs/in/raw/.svn/entries",
        "update_without_exclude_vcs/in/raw/.hg/hgrc",
        "update_without_exclude_vcs/in/raw/.hgignore",
        "update_without_exclude_vcs/in/raw/.bzr/branch-format",
        "update_without_exclude_vcs/in/raw/.bzrignore",
        "update_without_exclude_vcs/in/raw/CVS/Root",
        "update_without_exclude_vcs/in/raw/.gitmodules",
        "update_without_exclude_vcs/in/raw/.gitattributes",
    ];
    for file in vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }
    let regular_files = [
        "update_without_exclude_vcs/in/raw/regular.txt",
        "update_without_exclude_vcs/in/raw/data.csv",
        "update_without_exclude_vcs/in/raw/document.pdf",
    ];
    for file in regular_files {
        fs::write(file, "regular file content").unwrap();
    }
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_without_exclude_vcs/update_without_exclude_vcs.pna",
        "--overwrite",
        "update_without_exclude_vcs/in/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    // update without exclude-vcs
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_without_exclude_vcs/update_without_exclude_vcs.pna",
        "update_without_exclude_vcs/in/raw/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "update_without_exclude_vcs/update_without_exclude_vcs.pna",
        "--overwrite",
        "--out-dir",
        "update_without_exclude_vcs/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    diff(
        "update_without_exclude_vcs/in/",
        "update_without_exclude_vcs/out/",
    )
    .unwrap();
}
