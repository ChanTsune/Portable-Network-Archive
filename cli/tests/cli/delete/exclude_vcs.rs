use crate::utils::{self, diff::diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn delete_with_exclude_vcs() {
    setup();
    TestResources::extract_in("raw/", "delete_with_exclude_vcs/in/").unwrap();
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
    let regular_files = [
        "delete_with_exclude_vcs/in/raw/regular.txt",
        "delete_with_exclude_vcs/in/raw/data.csv",
        "delete_with_exclude_vcs/in/raw/document.pdf",
    ];
    for file in regular_files {
        fs::write(file, "regular file content").unwrap();
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
    // delete with exclude-vcs
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "delete_with_exclude_vcs/delete_with_exclude_vcs.pna",
        "raw/",
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
        "delete_with_exclude_vcs/delete_with_exclude_vcs.pna",
        "--overwrite",
        "--out-dir",
        "delete_with_exclude_vcs/out/",
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
        "delete_with_exclude_vcs/in/",
        "delete_with_exclude_vcs/out/",
    )
    .unwrap();
}

#[test]
fn delete_without_exclude_vcs() {
    setup();
    TestResources::extract_in("raw/", "delete_without_exclude_vcs/in/").unwrap();
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
    let regular_files = [
        "delete_without_exclude_vcs/in/raw/regular.txt",
        "delete_without_exclude_vcs/in/raw/data.csv",
        "delete_without_exclude_vcs/in/raw/document.pdf",
    ];
    for file in regular_files {
        fs::write(file, "regular file content").unwrap();
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
    // delete without exclude-vcs
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "delete",
        "delete_without_exclude_vcs/delete_without_exclude_vcs.pna",
        "raw/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "delete_without_exclude_vcs/delete_without_exclude_vcs.pna",
        "--overwrite",
        "--out-dir",
        "delete_without_exclude_vcs/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    diff(
        "delete_without_exclude_vcs/in/",
        "delete_without_exclude_vcs/out/",
    )
    .unwrap();
}
