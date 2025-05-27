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
fn create_archive_respect_gitignore() {
    setup();
    let resources = TestResources::extract_in("create_gitignore");
    // base_dir_for_test_case is e.g., /app/target/tmp/pid/create_gitignore
    let base_dir_for_test_case = resources.path();

    // This will be the path given to `pna create` command, relative to `create_cwd`.
    // e.g., "create_gitignore/src_dir"
    let source_dir_name_relative_to_create_cwd = Path::new(base_dir_for_test_case.file_name().unwrap()).join("src_dir");

    // This is the full absolute path to the source directory where files and .gitignore reside.
    // e.g., /app/target/tmp/pid/create_gitignore/src_dir
    let full_src_dir_path = base_dir_for_test_case.join("src_dir");
    let full_src_subdir_path = full_src_dir_path.join("subdir");
    let full_src_temp_path = full_src_dir_path.join("temp");

    fs::create_dir_all(&full_src_subdir_path).unwrap();
    fs::create_dir_all(&full_src_temp_path).unwrap();

    // Create .gitignore file
    let gitignore_path = full_src_dir_path.join(".gitignore");
    let mut gitignore_file = fs::File::create(&gitignore_path).unwrap();
    writeln!(gitignore_file, "*.log").unwrap();
    writeln!(gitignore_file, "temp/").unwrap();
    writeln!(gitignore_file, "another_ignored_file.txt").unwrap();

    // Create files that should be versioned
    fs::write(full_src_dir_path.join("important_doc.txt"), "This is an important document.").unwrap();
    fs::write(full_src_subdir_path.join("another_doc.md"), "Another important document in a subdirectory.").unwrap();

    // Create files and directories that should be ignored
    fs::write(full_src_dir_path.join("debug.log"), "This is a debug log.").unwrap(); // Ignored by *.log
    fs::write(full_src_temp_path.join("cache.dat"), "This is a cache file in temp dir.").unwrap(); // Ignored by temp/
    fs::write(full_src_dir_path.join("another_ignored_file.txt"), "This file should be ignored by its name.").unwrap(); // Ignored by another_ignored_file.txt
    fs::write(full_src_subdir_path.join("report.log"), "This is a report log in subdir.").unwrap(); // Ignored by *.log

    // Define archive path and output directories
    let archive_path = base_dir_for_test_case.join("archive_gitignore.pna");
    let expected_out_dir = base_dir_for_test_case.join("expected_out");
    let actual_out_dir = base_dir_for_test_case.join("actual_out");

    fs::create_dir_all(expected_out_dir.join("subdir")).unwrap(); // For expected subdir/another_doc.md
    fs::create_dir_all(&actual_out_dir).unwrap();

    // Populate expected_out_dir (files that should NOT be ignored)
    fs::copy(full_src_dir_path.join("important_doc.txt"), expected_out_dir.join("important_doc.txt")).unwrap();
    fs::copy(full_src_subdir_path.join("another_doc.md"), expected_out_dir.join("subdir").join("another_doc.md")).unwrap();
    // .gitignore itself is usually not included unless explicitly added, and --gitignore flag controls behavior, not inclusion of the file.

    // CWD for invoking `pna create` will be one level above base_dir_for_test_case.
    // e.g. /app/target/tmp/pid
    let create_cwd = base_dir_for_test_case.parent().unwrap();
    let pna_executable = Path::new(env!("CARGO_BIN_EXE_pna"));

    // Run pna create
    let mut cmd_create = StdCommand::new(&pna_executable);
    cmd_create
        .current_dir(&create_cwd) // Set CWD to /app/target/tmp/pid/
        .arg("create")
        .arg(&archive_path) // Absolute path to archive: /app/target/tmp/pid/create_gitignore/archive_gitignore.pna
        .arg(&source_dir_name_relative_to_create_cwd) // Source path: "create_gitignore/src_dir"
        .arg("--gitignore")
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
        "--strip-components", "2", // To strip "create_gitignore/src_dir/"
        "--quiet",
    ];
    let cli_extract = cli::Cli::parse_from(cli_args_extract);
    cli_extract.exec().unwrap();

    // Compare actual_out_dir and expected_out_dir
    let diff_output = diff(&expected_out_dir, &actual_out_dir);
    assert!(
        diff_output.is_empty(),
        "Diff output should be empty. Diff:\nExpected:\n{}\nActual:\n{}",
        tree_string(&expected_out_dir), tree_string(&actual_out_dir)
    );
}

// Helper function to visualize directory trees for debugging diffs
fn tree_string(dir: &Path) -> String {
    let mut result = String::new();
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    result.push_str(&format!("  {}\n", path.file_name().unwrap_or_default().to_string_lossy()));
                    if path.is_dir() {
                        result.push_str(&tree_string_recursive(&path, "    "));
                    }
                }
            }
        }
        Err(e) => result.push_str(&format!("Error reading dir {:?}: {}\n", dir, e)),
    }
    result
}

fn tree_string_recursive(dir: &Path, prefix: &str) -> String {
    let mut result = String::new();
    match std::fs::read_dir(dir) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    result.push_str(&format!("{}{}\n", prefix, path.file_name().unwrap_or_default().to_string_lossy()));
                    if path.is_dir() {
                        result.push_str(&tree_string_recursive(&path, &format!("{}  ", prefix)));
                    }
                }
            }
        }
        Err(e) => result.push_str(&format!("{}Error reading dir {:?}: {}\n", prefix, dir, e)),
    }
    result
}
