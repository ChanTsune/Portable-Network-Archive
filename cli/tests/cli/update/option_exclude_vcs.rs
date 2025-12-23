use crate::utils::{EmbedExt, TestResources, archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{collections::HashSet, fs};

/// Precondition: An archive exists without VCS files.
/// Action: Add VCS files to source, run `pna experimental update` with `--exclude-vcs`.
/// Expectation: New VCS files are not added to the archive; regular files are added.
#[test]
fn update_with_exclude_vcs() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_option_exclude_vcs");
    TestResources::extract_in("raw/", "update_option_exclude_vcs/in/").unwrap();

    // Create initial archive (no VCS files yet)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_option_exclude_vcs/archive.pna",
        "--overwrite",
        "update_option_exclude_vcs/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Now add VCS files to source
    let vcs_files = [
        "update_option_exclude_vcs/in/raw/.git/HEAD",
        "update_option_exclude_vcs/in/raw/.gitignore",
        "update_option_exclude_vcs/in/raw/.svn/entries",
    ];
    for file in &vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }

    // Add a new regular file
    fs::write(
        "update_option_exclude_vcs/in/raw/new_regular.txt",
        "new regular content",
    )
    .unwrap();

    // Run update with --exclude-vcs
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_option_exclude_vcs/archive.pna",
        "update_option_exclude_vcs/in/",
        "--unstable",
        "--exclude-vcs",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive contents
    let mut seen = HashSet::new();
    archive::for_each_entry("update_option_exclude_vcs/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // VCS files should NOT be in archive
    for vcs_file in &vcs_files {
        let vcs_path = vcs_file.trim_start_matches("update_option_exclude_vcs/in/");
        assert!(
            !seen.iter().any(|p| p.ends_with(vcs_path)),
            "VCS file should not be added to archive with --exclude-vcs: {vcs_path}"
        );
    }

    // New regular file SHOULD be in archive
    assert!(
        seen.iter().any(|p| p.ends_with("raw/new_regular.txt")),
        "new regular file should be added to archive"
    );
}

/// Precondition: An archive exists without VCS files.
/// Action: Add VCS files to source, run `pna experimental update` without `--exclude-vcs`.
/// Expectation: VCS files are added to the archive.
#[test]
fn update_without_exclude_vcs() {
    setup();
    // Clean up any leftover files from previous test runs
    let _ = fs::remove_dir_all("update_option_no_exclude_vcs");
    TestResources::extract_in("raw/", "update_option_no_exclude_vcs/in/").unwrap();

    // Create initial archive (no VCS files yet)
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "update_option_no_exclude_vcs/archive.pna",
        "--overwrite",
        "update_option_no_exclude_vcs/in/",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Now add VCS files to source
    let vcs_files = [
        "update_option_no_exclude_vcs/in/raw/.git/HEAD",
        "update_option_no_exclude_vcs/in/raw/.gitignore",
    ];
    for file in &vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }

    // Run update WITHOUT --exclude-vcs
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "update",
        "-f",
        "update_option_no_exclude_vcs/archive.pna",
        "update_option_no_exclude_vcs/in/",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    // Verify archive contents
    let mut seen = HashSet::new();
    archive::for_each_entry("update_option_no_exclude_vcs/archive.pna", |entry| {
        seen.insert(entry.header().path().to_string());
    })
    .unwrap();

    // VCS files SHOULD be in archive (no exclusion)
    for vcs_file in &vcs_files {
        let vcs_path = vcs_file.trim_start_matches("update_option_no_exclude_vcs/in/");
        assert!(
            seen.iter().any(|p| p.ends_with(vcs_path)),
            "VCS file should be added to archive without --exclude-vcs: {vcs_path}"
        );
    }
}
