use crate::utils::{diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{
    fs,
    path::Path,
    process::{Command as StdCommand, Stdio},
};

#[test]
fn create_archive_include_pattern() {
    setup();
    let resources = TestResources::extract_in("create_include");
    // base_dir_for_test_case is e.g., /app/target/tmp/pid/create_include
    let base_dir_for_test_case = resources.path(); 

    // This will be the path given to `pna create` command, relative to `create_cwd`.
    // e.g., "create_include/in"
    let source_dir_relative_to_create_cwd = Path::new(base_dir_for_test_case.file_name().unwrap()).join("in");
    
    // This is the full absolute path to the input files.
    // e.g., /app/target/tmp/pid/create_include/in
    let full_input_dir_path = base_dir_for_test_case.join("in"); 
    let full_input_sub_dir_path = full_input_dir_path.join("sub");
    fs::create_dir_all(&full_input_sub_dir_path).unwrap();

    // Populate input directory with actual files
    fs::write(full_input_dir_path.join("apple.txt"), "Apple content").unwrap();
    fs::write(full_input_dir_path.join("apricot.log"), "Apricot log").unwrap();
    fs::write(full_input_dir_path.join("banana.txt"), "Banana content").unwrap();
    fs::write(full_input_sub_dir_path.join("orange.txt"), "Orange content").unwrap();
    fs::write(full_input_sub_dir_path.join("kiwi.log"), "Kiwi log").unwrap();

    // CWD for invoking `pna create` will be one level above base_dir_for_test_case.
    // e.g. /app/target/tmp/pid
    let create_cwd = base_dir_for_test_case.parent().unwrap();
    let pna_executable = Path::new(env!("CARGO_BIN_EXE_pna"));

    // --- Test 1: Basic Include ---
    let archive_path_1 = base_dir_for_test_case.join("archive1.pna"); // Store archive inside create_include/
    let expected_out_1 = base_dir_for_test_case.join("expected_out_1");
    let actual_out_1 = base_dir_for_test_case.join("actual_out_1");
    fs::create_dir_all(&expected_out_1).unwrap();
    fs::create_dir_all(expected_out_1.join("sub")).unwrap(); // For sub/orange.txt
    fs::create_dir_all(&actual_out_1).unwrap();

    // Populate expected_out_1 (paths after stripping "create_include/in/")
    fs::copy(full_input_dir_path.join("apple.txt"), expected_out_1.join("apple.txt")).unwrap();
    fs::copy(full_input_dir_path.join("banana.txt"), expected_out_1.join("banana.txt")).unwrap();
    fs::copy(full_input_sub_dir_path.join("orange.txt"), expected_out_1.join("sub").join("orange.txt")).unwrap();

    // Run pna create for Test 1
    let mut cmd_create_1 = StdCommand::new(&pna_executable);
    cmd_create_1
        .current_dir(&create_cwd) // Set CWD to /app/target/tmp/pid/
        .arg("create")
        .arg(&archive_path_1) // Absolute path to archive: /app/target/tmp/pid/create_include/archive1.pna
        .arg(&source_dir_relative_to_create_cwd) // Source path: "create_include/in"
        .arg("--include")
        .arg("*.txt")
        .arg("--overwrite")
        .arg("--unstable")
        .arg("--quiet")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    
    let status_create_1 = cmd_create_1.status().expect("Failed to execute pna create for test 1");
    assert!(status_create_1.success(), "pna create for test 1 failed. Stderr: {:?}", cmd_create_1.output().map(|o| String::from_utf8_lossy(&o.stderr).into_owned()));


    // Run pna x for Test 1
    let cli_args_extract_1 = vec![
        "pna",
        "x",
        archive_path_1.to_str().unwrap(),
        "--out-dir",
        actual_out_1.to_str().unwrap(),
        "--strip-components", "2", // To strip "create_include/in/"
        "--quiet",
    ];
    let cli_extract_1 = cli::Cli::parse_from(cli_args_extract_1);
    cli_extract_1.exec().unwrap();

    let diff_output_1 = diff(&expected_out_1, &actual_out_1);
    assert!(diff_output_1.is_empty(), "Test 1 Diff: expected empty, got:\n{}", diff_output_1);

    // --- Test 2: Include with Exclude Precedence ---
    let archive_path_2 = base_dir_for_test_case.join("archive2.pna"); // Store archive inside create_include/
    let expected_out_2 = base_dir_for_test_case.join("expected_out_2");
    let actual_out_2 = base_dir_for_test_case.join("actual_out_2");
    fs::create_dir_all(&expected_out_2).unwrap();
    fs::create_dir_all(&actual_out_2).unwrap();

    // Populate expected_out_2 (sub/orange.txt is excluded by "sub/*")
    fs::copy(full_input_dir_path.join("apple.txt"), expected_out_2.join("apple.txt")).unwrap();
    fs::copy(full_input_dir_path.join("banana.txt"), expected_out_2.join("banana.txt")).unwrap();

    // Run pna create for Test 2
    let mut cmd_create_2 = StdCommand::new(&pna_executable);
    cmd_create_2
        .current_dir(&create_cwd) // Set CWD to /app/target/tmp/pid/
        .arg("create")
        .arg(&archive_path_2) // Absolute path to archive: /app/target/tmp/pid/create_include/archive2.pna
        .arg(&source_dir_relative_to_create_cwd) // Source path: "create_include/in"
        .arg("--include")
        .arg("*.txt")
        .arg("--exclude") // This should apply to files within "create_include/in/"
        .arg("sub/*") 
        .arg("--overwrite")
        .arg("--unstable")
        .arg("--quiet")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let status_create_2 = cmd_create_2.status().expect("Failed to execute pna create for test 2");
    assert!(status_create_2.success(), "pna create for test 2 failed. Stderr: {:?}", cmd_create_2.output().map(|o| String::from_utf8_lossy(&o.stderr).into_owned()));
    
    // Run pna x for Test 2
    let cli_args_extract_2 = vec![
        "pna",
        "x",
        archive_path_2.to_str().unwrap(),
        "--out-dir",
        actual_out_2.to_str().unwrap(),
        "--strip-components", "2", // To strip "create_include/in/"
        "--quiet",
    ];
    let cli_extract_2 = cli::Cli::parse_from(cli_args_extract_2);
    cli_extract_2.exec().unwrap();

    let diff_output_2 = diff(&expected_out_2, &actual_out_2);
    assert!(diff_output_2.is_empty(), "Test 2 Diff: expected empty, got:\n{}", diff_output_2);
}
