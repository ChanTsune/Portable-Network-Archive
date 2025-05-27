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
fn create_archive_files_from_stdin() {
    setup();
    let resources = TestResources::extract_in("create_files_from_stdin");
    let base_path = resources.path(); // Base path for test case, e.g., target/tmp/xxx/create_files_from_stdin

    // 1. Create input directory and files
    let input_dir = base_path.join("in");
    let subdir_in_input = input_dir.join("subdir");
    fs::create_dir_all(&subdir_in_input).unwrap();

    let path_item_a = input_dir.join("itemA.txt");
    let path_item_b = input_dir.join("itemB.log"); // This item will not be in stdin
    let path_item_c = subdir_in_input.join("itemC.dat");

    fs::write(&path_item_a, "Content of itemA").unwrap();
    fs::write(&path_item_b, "Content of itemB (should not be archived)").unwrap();
    fs::write(&path_item_c, "Content of itemC").unwrap();

    // 2. Prepare a string containing paths relative to the execution root (`/app`)
    let current_dir = std::env::current_dir().unwrap(); // Should be /app

    let path_in_list_a = path_item_a.strip_prefix(&current_dir).unwrap().to_str().unwrap();
    let path_in_list_c = path_item_c.strip_prefix(&current_dir).unwrap().to_str().unwrap();

    let files_to_archive_str = format!("{}\n{}", path_in_list_a, path_in_list_c);

    // 3. Define archive path and output directories
    let archive_path = base_path.join("archive_stdin.pna");
    let expected_out_dir = base_path.join("expected_out");
    let actual_out_dir = base_path.join("actual_out");
    fs::create_dir_all(&expected_out_dir).unwrap();
    fs::create_dir_all(&actual_out_dir).unwrap();

    // 4. Populate expected_out_dir
    // Files will be extracted with paths relative to repo root.
    let expected_file_a_path = expected_out_dir.join(path_in_list_a);
    let expected_file_c_path = expected_out_dir.join(path_in_list_c);

    fs::create_dir_all(expected_file_a_path.parent().unwrap()).unwrap();
    fs::copy(&path_item_a, &expected_file_a_path).unwrap();
    fs::create_dir_all(expected_file_c_path.parent().unwrap()).unwrap();
    fs::copy(&path_item_c, &expected_file_c_path).unwrap();

    // 5. Run `pna create` with stdin input
    let pna_executable = Path::new(env!("CARGO_BIN_EXE_pna"));
    let mut create_cmd = StdCommand::new(pna_executable);
    create_cmd
        .arg("create")
        .arg(archive_path.to_str().unwrap())
        .arg("--files-from-stdin")
        .arg("--overwrite")
        .arg("--unstable")
        .arg("--quiet")
        .stdin(Stdio::piped()) // Pipe stdin
        .stdout(Stdio::null()) // Suppress stdout for cleaner test run
        .stderr(Stdio::null()); // Suppress stderr for cleaner test run

    let mut child = create_cmd.spawn().expect("Failed to spawn pna create process");
    let mut child_stdin = child.stdin.take().expect("Failed to open stdin for pna create");
    // Write paths to pna's stdin in a separate thread to avoid deadlocks
    std::thread::spawn(move || {
        child_stdin
            .write_all(files_to_archive_str.as_bytes())
            .expect("Failed to write to pna create stdin");
    });

    let status = child.wait().expect("pna create process failed to run");
    assert!(status.success(), "pna create --files-from-stdin failed");

    // 6. Run `pna x` to extract
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

    // 7. Compare actual_out_dir and expected_out_dir
    let diff_output = diff(&expected_out_dir, &actual_out_dir);
    assert!(
        diff_output.is_empty(),
        "Diff output should be empty. Diff:\n{}",
        diff_output
    );
}
