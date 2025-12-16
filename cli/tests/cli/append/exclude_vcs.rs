use crate::utils::{self, EmbedExt, TestResources, diff::diff, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::fs;

#[test]
fn append_with_exclude_vcs() {
    setup();
    TestResources::extract_in("raw/", "append_with_exclude_vcs/in/").unwrap();
    let vcs_files = [
        "append_with_exclude_vcs/in/raw/.git/HEAD",
        "append_with_exclude_vcs/in/raw/.git/config",
        "append_with_exclude_vcs/in/raw/.gitignore",
        "append_with_exclude_vcs/in/raw/.svn/entries",
        "append_with_exclude_vcs/in/raw/.hg/hgrc",
        "append_with_exclude_vcs/in/raw/.hgignore",
        "append_with_exclude_vcs/in/raw/.bzr/branch-format",
        "append_with_exclude_vcs/in/raw/.bzrignore",
        "append_with_exclude_vcs/in/raw/CVS/Root",
        "append_with_exclude_vcs/in/raw/.gitmodules",
        "append_with_exclude_vcs/in/raw/.gitattributes",
    ];
    for file in vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }
    let regular_files = [
        "append_with_exclude_vcs/in/raw/regular.txt",
        "append_with_exclude_vcs/in/raw/data.csv",
        "append_with_exclude_vcs/in/raw/document.pdf",
    ];
    for file in regular_files {
        fs::write(file, "regular file content").unwrap();
    }
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "append_with_exclude_vcs/append_with_exclude_vcs.pna",
        "--overwrite",
        "append_with_exclude_vcs/in/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    // append with exclude-vcs
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "a",
        "append_with_exclude_vcs/append_with_exclude_vcs.pna",
        "append_with_exclude_vcs/in/",
        "--exclude-vcs",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "append_with_exclude_vcs/append_with_exclude_vcs.pna",
        "--overwrite",
        "--out-dir",
        "append_with_exclude_vcs/out/",
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
        "append_with_exclude_vcs/in/",
        "append_with_exclude_vcs/out/",
    )
    .unwrap();
}

#[test]
fn append_without_exclude_vcs() {
    setup();
    TestResources::extract_in("raw/", "append_without_exclude_vcs/in/").unwrap();
    let vcs_files = [
        "append_without_exclude_vcs/in/raw/.git/HEAD",
        "append_without_exclude_vcs/in/raw/.git/config",
        "append_without_exclude_vcs/in/raw/.gitignore",
        "append_without_exclude_vcs/in/raw/.svn/entries",
        "append_without_exclude_vcs/in/raw/.hg/hgrc",
        "append_without_exclude_vcs/in/raw/.hgignore",
        "append_without_exclude_vcs/in/raw/.bzr/branch-format",
        "append_without_exclude_vcs/in/raw/.bzrignore",
        "append_without_exclude_vcs/in/raw/CVS/Root",
        "append_without_exclude_vcs/in/raw/.gitmodules",
        "append_without_exclude_vcs/in/raw/.gitattributes",
    ];
    for file in vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }
    let regular_files = [
        "append_without_exclude_vcs/in/raw/regular.txt",
        "append_without_exclude_vcs/in/raw/data.csv",
        "append_without_exclude_vcs/in/raw/document.pdf",
    ];
    for file in regular_files {
        fs::write(file, "regular file content").unwrap();
    }
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "append_without_exclude_vcs/append_without_exclude_vcs.pna",
        "--overwrite",
        "append_without_exclude_vcs/in/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    // append without exclude-vcs
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "a",
        "append_without_exclude_vcs/append_without_exclude_vcs.pna",
        "append_without_exclude_vcs/in/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "append_without_exclude_vcs/append_without_exclude_vcs.pna",
        "--overwrite",
        "--out-dir",
        "append_without_exclude_vcs/out/",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();
    diff(
        "append_without_exclude_vcs/in/",
        "append_without_exclude_vcs/out/",
    )
    .unwrap();
}
