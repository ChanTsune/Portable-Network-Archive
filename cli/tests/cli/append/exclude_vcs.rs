use crate::utils::{EmbedExt, TestResources, archive, diff::assert_dirs_equal, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::collections::HashSet;
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
    // Record entry counts before append
    let mut before_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    archive::for_each_entry(
        "append_with_exclude_vcs/append_with_exclude_vcs.pna",
        |entry| {
            *before_counts
                .entry(entry.header().path().to_string())
                .or_insert(0) += 1;
        },
    )
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
    // Record entry counts after append
    let mut after_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    archive::for_each_entry(
        "append_with_exclude_vcs/append_with_exclude_vcs.pna",
        |entry| {
            *after_counts
                .entry(entry.header().path().to_string())
                .or_insert(0) += 1;
        },
    )
    .unwrap();
    // Identify entries whose count increased (i.e., were appended)
    let mut added_paths = HashSet::new();
    for (path, after_count) in &after_counts {
        let before_count = before_counts.get(path).copied().unwrap_or(0);
        if *after_count > before_count {
            added_paths.insert(path.clone());
        }
    }
    // Verify appended entries do not contain VCS paths
    for entry in &added_paths {
        assert!(
            !is_vcs_path(entry),
            "VCS file should not be appended: {entry}"
        );
    }
    assert!(
        !added_paths.is_empty(),
        "append should add at least one entry"
    );
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
    assert_dirs_equal(
        "append_without_exclude_vcs/in/",
        "append_without_exclude_vcs/out/",
    );
}

fn is_vcs_path(path: &str) -> bool {
    let vcs_indicators = [
        ".git/",
        ".svn/",
        ".hg/",
        ".bzr/",
        "CVS/",
        ".gitignore",
        ".gitmodules",
        ".gitattributes",
        ".hgignore",
        ".bzrignore",
    ];
    vcs_indicators
        .iter()
        .any(|indicator| path.contains(indicator))
        || path.ends_with(".git")
}
