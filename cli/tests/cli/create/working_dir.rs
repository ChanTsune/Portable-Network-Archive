use crate::utils::{diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::fs;
use std::path::Path;

#[test]
fn create_archive_change_working_dir() {
    setup();
    let resources = TestResources::extract_in("create_cd");
    // base_test_dir is e.g., /app/target/tmp/pid/create_cd
    let base_test_dir = resources.path();

    // Directory to change into using -C
    // e.g., /app/target/tmp/pid/create_cd/base_dir/actual_files
    let actual_files_dir = base_test_dir.join("base_dir").join("actual_files");
    fs::create_dir_all(&actual_files_dir).unwrap();

    // Directory for a file that should not be included if -C works correctly
    let other_dir = base_test_dir.join("other_dir");
    fs::create_dir_all(&other_dir).unwrap();

    // Create source files
    let content_file1 = "Content of file1 for -C test.";
    let content_file2 = "Content of file2 for -C test.";
    fs::write(actual_files_dir.join("file1.txt"), content_file1).unwrap();
    fs::write(actual_files_dir.join("file2.txt"), content_file2).unwrap();
    fs::write(other_dir.join("ignored_file.txt"), "This file should be ignored.").unwrap();

    // Define archive path and output directories
    let archive_path = base_test_dir.join("archive_cd.pna");
    let expected_out_dir = base_test_dir.join("expected_out");
    let actual_out_dir = base_test_dir.join("actual_out");

    fs::create_dir_all(&expected_out_dir).unwrap();
    fs::create_dir_all(&actual_out_dir).unwrap();

    // Populate expected_out_dir (files should be at the root of the archive)
    fs::write(expected_out_dir.join("file1.txt"), content_file1).unwrap();
    fs::write(expected_out_dir.join("file2.txt"), content_file2).unwrap();

    // Run `pna create`
    // pna create <archive_path> -C <path_to_actual_files_dir> file1.txt file2.txt --overwrite --quiet
    // The paths "file1.txt" and "file2.txt" are relative to the new CWD set by -C
    let cli_args_create = vec![
        "pna",
        "create",
        archive_path.to_str().unwrap(),
        "-C",
        actual_files_dir.to_str().unwrap(),
        "file1.txt",
        "file2.txt",
        "--overwrite",
        "--quiet",
    ];
    let cli_create = cli::Cli::parse_from(cli_args_create);
    cli_create.exec().unwrap();

    // Run `pna x` (extract)
    // No --strip-components needed as paths should be archived relative to the -C directory.
    let cli_args_extract = vec![
        "pna",
        "x",
        archive_path.to_str().unwrap(),
        "--out-dir",
        actual_out_dir.to_str().unwrap(),
        "--quiet",
    ];
    let cli_extract = cli::Cli::parse_from(cli_args_extract);
    cli_extract.exec().unwrap();

    // Compare actual_out_dir and expected_out_dir
    let diff_output = diff(&expected_out_dir, &actual_out_dir);
    assert!(
        diff_output.is_empty(),
        "Diff output should be empty. Diff:\n{}",
        diff_output
    );
}
