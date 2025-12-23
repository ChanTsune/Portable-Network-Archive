use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

/// Precondition: A directory contains VCS files (`.git/`, `.svn/`, `.hg/`, `.bzr/`, `CVS/`, `.gitignore`, etc.) and regular files.
/// Action: Run `pna create` with `--exclude-vcs`.
/// Expectation: All VCS-related files and directories are excluded; regular files are included.
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

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "create_with_exclude_vcs/create_with_exclude_vcs.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    // Verify included entries (regular files + original raw resources)
    let required_entries = [
        // Regular files we created
        "create_with_exclude_vcs/in/raw/regular.txt",
        "create_with_exclude_vcs/in/raw/data.csv",
        "create_with_exclude_vcs/in/raw/document.pdf",
        // Original test resources (non-VCS)
        "create_with_exclude_vcs/in/raw/empty.txt",
        "create_with_exclude_vcs/in/raw/text.txt",
        "create_with_exclude_vcs/in/raw/first/second/third/pna.txt",
        "create_with_exclude_vcs/in/raw/parent/child.txt",
        "create_with_exclude_vcs/in/raw/images/icon.bmp",
        "create_with_exclude_vcs/in/raw/images/icon.png",
        "create_with_exclude_vcs/in/raw/images/icon.svg",
        "create_with_exclude_vcs/in/raw/pna/empty.pna",
        "create_with_exclude_vcs/in/raw/pna/nest.pna",
    ];
    for required in required_entries {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }

    // Verify excluded entries (VCS files)
    for vcs_file in vcs_files {
        assert!(
            !seen.contains(vcs_file),
            "VCS entry should not be present: {vcs_file}"
        );
    }

    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}

/// Precondition: A directory contains VCS files (`.git/`, `.svn/`, `.hg/`, `.bzr/`, `CVS/`, `.gitignore`, etc.) and regular files.
/// Action: Run `pna create` without `--exclude-vcs`.
/// Expectation: All files including VCS files are included in the archive.
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

    let mut seen = HashSet::new();
    archive::for_each_entry(
        "create_without_exclude_vcs/create_without_exclude_vcs.pna",
        |entry| {
            seen.insert(entry.header().path().to_string());
        },
    )
    .unwrap();

    // Verify all entries are included (VCS files + regular files + original resources)
    let required_entries = [
        // VCS files we created
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
        // Regular files we created
        "create_without_exclude_vcs/in/raw/regular.txt",
        "create_without_exclude_vcs/in/raw/data.csv",
        "create_without_exclude_vcs/in/raw/document.pdf",
        // Original test resources
        "create_without_exclude_vcs/in/raw/empty.txt",
        "create_without_exclude_vcs/in/raw/text.txt",
        "create_without_exclude_vcs/in/raw/first/second/third/pna.txt",
        "create_without_exclude_vcs/in/raw/parent/child.txt",
        "create_without_exclude_vcs/in/raw/images/icon.bmp",
        "create_without_exclude_vcs/in/raw/images/icon.png",
        "create_without_exclude_vcs/in/raw/images/icon.svg",
        "create_without_exclude_vcs/in/raw/pna/empty.pna",
        "create_without_exclude_vcs/in/raw/pna/nest.pna",
    ];
    for required in required_entries {
        assert!(
            seen.take(required).is_some(),
            "required entry missing: {required}"
        );
    }

    assert!(seen.is_empty(), "unexpected entries found: {seen:?}");
}
