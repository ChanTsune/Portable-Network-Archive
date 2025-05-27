use crate::utils::{diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;

#[test]
fn create_solid_archive_default_compression() {
    setup();
    let resources = TestResources::extract_in("create_solid_default");
    let input_dir = resources.path_with_default("in");
    fs::create_dir_all(&input_dir).unwrap();

    // Populate with small files
    fs::write(input_dir.join("file_a.txt"), "Content of file A.").unwrap();
    fs::write(input_dir.join("file_b.txt"), "Content of file B, which is slightly different.").unwrap();
    fs::write(input_dir.join("file_c.txt"), "Third file for solid archive testing.").unwrap();

    let archive_path = resources.path_with_default("solid_default.pna");
    let extract_dir = resources.path_with_default("out");
    fs::create_dir_all(&extract_dir).unwrap();

    // Create solid archive (default compression: Zstd)
    let cli_args_create = vec![
        "pna",
        "create",
        archive_path.to_str().unwrap(),
        input_dir.to_str().unwrap(),
        "--solid",
        "--overwrite",
        "--quiet",
    ];
    let cli_create = cli::Cli::parse_from(cli_args_create);
    cli_create.exec().unwrap();

    // Extract archive
    let cli_args_extract = vec![
        "pna",
        "x",
        archive_path.to_str().unwrap(),
        "--out-dir",
        extract_dir.to_str().unwrap(),
        "--quiet",
    ];
    let cli_extract = cli::Cli::parse_from(cli_args_extract);
    cli_extract.exec().unwrap();

    // Compare directories
    let diff_output = diff(&input_dir, &extract_dir);
    assert!(diff_output.is_empty(), "Diff output should be empty for default compression. Diff:\n{}", diff_output);
}

#[test]
fn create_solid_archive_store() {
    setup();
    let resources = TestResources::extract_in("create_solid_store");
    let input_dir = resources.path_with_default("in");
    fs::create_dir_all(&input_dir).unwrap();

    // Populate with small files (can be same as above, or different for variety)
    fs::write(input_dir.join("doc1.txt"), "Document one for solid store test.").unwrap();
    fs::write(input_dir.join("doc2.txt"), "Document two, also part of the test.").unwrap();
    fs::write(input_dir.join("doc3.txt"), "Final document in this set.").unwrap();

    let archive_path = resources.path_with_default("solid_store.pna");
    let extract_dir = resources.path_with_default("out");
    fs::create_dir_all(&extract_dir).unwrap();

    // Create solid archive with --store (no compression)
    let cli_args_create = vec![
        "pna",
        "create",
        archive_path.to_str().unwrap(),
        input_dir.to_str().unwrap(),
        "--solid",
        "--store",
        "--overwrite",
        "--quiet",
    ];
    let cli_create = cli::Cli::parse_from(cli_args_create);
    cli_create.exec().unwrap();

    // Extract archive
    let cli_args_extract = vec![
        "pna",
        "x",
        archive_path.to_str().unwrap(),
        "--out-dir",
        extract_dir.to_str().unwrap(),
        "--quiet",
    ];
    let cli_extract = cli::Cli::parse_from(cli_args_extract);
    cli_extract.exec().unwrap();

    // Compare directories
    let diff_output = diff(&input_dir, &extract_dir);
    assert!(diff_output.is_empty(), "Diff output should be empty for --store. Diff:\n{}", diff_output);
}
