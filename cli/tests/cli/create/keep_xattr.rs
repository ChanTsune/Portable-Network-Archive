use crate::utils::{setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{fs, path::Path};
use xattr;

#[test]
fn create_keep_xattr() {
    setup();
    let resources = TestResources::extract_in("test_xattr_resources");
    let source_file_path = resources.path().join("test_file.txt");
    fs::write(&source_file_path, "This is a test file.").unwrap();

    // Set extended attribute
    let attr_name = "user.pna_test_attr";
    let attr_value = "test_value";
    xattr::set(&source_file_path, attr_name, attr_value.as_bytes()).unwrap();

    let archive_path = resources.path().join("test_archive.pna");
    let cli_archive_path = archive_path.clone();
    let extract_dir = resources.path().join("extracted_files");
    fs::create_dir_all(&extract_dir).unwrap();

    // Create archive with --keep-xattr
    let cli_args = vec![
        "pna",
        "create",
        cli_archive_path.to_str().unwrap(),
        source_file_path.to_str().unwrap(),
        "--keep-xattr",
        "--unstable",
    ];
    let cli = cli::Cli::parse_from(cli_args);
    cli.exec().unwrap();

    // Extract archive
    let cli_args_extract = vec![
        "pna",
        "extract",
        cli_archive_path.to_str().unwrap(),
        "--out-dir",
        extract_dir.to_str().unwrap(),
    ];
    let cli_extract = cli::Cli::parse_from(cli_args_extract);
    cli_extract.exec().unwrap();

    // Verify extended attribute
    let extracted_file_path = extract_dir.join("test_file.txt");
    let retrieved_attr_value = xattr::get(&extracted_file_path, attr_name)
        .unwrap()
        .unwrap();
    assert_eq!(retrieved_attr_value, attr_value.as_bytes());
}
