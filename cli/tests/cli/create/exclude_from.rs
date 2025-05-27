use crate::utils::{diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::{Command as StdCommand, Stdio},
};

#[test]
fn create_archive_exclude_from_file() {
    setup();
    let resources = TestResources::extract_in("create_exclude_from");
    // base_dir_for_test_case is e.g., /app/target/tmp/pid/create_exclude_from
    let base_dir_for_test_case = resources.path();

    // This will be the path given to `pna create` command, relative to `create_cwd`.
    // e.g., "create_exclude_from/in"
    let source_dir_relative_to_create_cwd = Path::new(base_dir_for_test_case.file_name().unwrap()).join("in");

    // This is the full absolute path to the input files.
    // e.g., /app/target/tmp/pid/create_exclude_from/in
    let full_input_dir_path = base_dir_for_test_case.join("in");
    let full_input_sub_dir_path = full_input_dir_path.join("sub_dir");
    fs::create_dir_all(&full_input_sub_dir_path).unwrap();

    // Populate input directory with actual files
    fs::write(full_input_dir_path.join("doc.txt"), "Document text").unwrap();
    fs::write(full_input_dir_path.join("image.jpg"), "JPG image data").unwrap();
    fs::write(full_input_dir_path.join("temp.tmp"), "Temporary file data").unwrap(); // Excluded by *.tmp
    fs::write(full_input_dir_path.join("archive.zip"), "ZIP archive data").unwrap();
    fs::write(full_input_sub_dir_path.join("another.txt"), "Another text file in sub_dir").unwrap(); // Excluded by sub_dir/*.txt
    fs::write(full_input_sub_dir_path.join("old.bak"), "Backup file data").unwrap(); // Excluded by *.bak

    // Create exclude_patterns.txt
    let exclude_file_path = base_dir_for_test_case.join("exclude_patterns.txt");
    let mut exclude_file = fs::File::create(&exclude_file_path).unwrap();
    writeln!(exclude_file, "*.tmp").unwrap();
    writeln!(exclude_file, "*.bak").unwrap();
    writeln!(exclude_file, "sub_dir/*.txt").unwrap(); // This pattern is relative to items in "create_exclude_from/in"

    // Define archive path and output directories
    let archive_path = base_dir_for_test_case.join("archive_exclude.pna");
    let expected_out_dir = base_dir_for_test_case.join("expected_out");
    let actual_out_dir = base_dir_for_test_case.join("actual_out");
    fs::create_dir_all(&expected_out_dir).unwrap();
    // fs::create_dir_all(expected_out_dir.join("sub_dir")).unwrap(); // Not needed as no files from sub_dir are expected
    fs::create_dir_all(&actual_out_dir).unwrap();

    // Populate expected_out_dir (files that should NOT be excluded)
    fs::copy(full_input_dir_path.join("doc.txt"), expected_out_dir.join("doc.txt")).unwrap();
    fs::copy(full_input_dir_path.join("image.jpg"), expected_out_dir.join("image.jpg")).unwrap();
    fs::copy(full_input_dir_path.join("archive.zip"), expected_out_dir.join("archive.zip")).unwrap();
    // temp.tmp, sub_dir/another.txt, and sub_dir/old.bak should be excluded.

    // CWD for invoking `pna create` will be one level above base_dir_for_test_case.
    // e.g. /app/target/tmp/pid
    let create_cwd = base_dir_for_test_case.parent().unwrap();
    let pna_executable = Path::new(env!("CARGO_BIN_EXE_pna"));

    // Run pna create
    let mut cmd_create = StdCommand::new(&pna_executable);
    cmd_create
        .current_dir(&create_cwd) // Set CWD to /app/target/tmp/pid/
        .arg("create")
        .arg(&archive_path) // Absolute path to archive: /app/target/tmp/pid/create_exclude_from/archive_exclude.pna
        .arg(&source_dir_relative_to_create_cwd) // Source path: "create_exclude_from/in"
        .arg("--exclude-from")
        .arg(&exclude_file_path) // Absolute path to exclude_patterns.txt
        .arg("--overwrite")
        .arg("--unstable")
        .arg("--quiet")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let status_create = cmd_create.status().expect("Failed to execute pna create");
    assert!(status_create.success(), "pna create failed. Stderr: {:?}", cmd_create.output().map(|o| String::from_utf8_lossy(&o.stderr).into_owned()));

    // Run pna x
    let cli_args_extract = vec![
        "pna",
        "x",
        archive_path.to_str().unwrap(),
        "--out-dir",
        actual_out_dir.to_str().unwrap(),
        "--strip-components", "2", // To strip "create_exclude_from/in/"
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
