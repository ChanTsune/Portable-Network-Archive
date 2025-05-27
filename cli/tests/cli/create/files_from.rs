use crate::utils::{diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

#[test]
fn create_archive_files_from() {
    setup();
    let resources = TestResources::extract_in("create_files_from");
    let base_path = resources.path(); // Base path for test case, e.g., target/tmp/xxx/create_files_from

    // 1. Create input directory and files
    let input_dir = base_path.join("in");
    let another_dir_in_input = input_dir.join("another_dir");
    fs::create_dir_all(&another_dir_in_input).unwrap();

    fs::write(input_dir.join("file1.txt"), "Content of file1").unwrap();
    fs::write(input_dir.join("file2.txt"), "Content of file2 (should not be archived)").unwrap();
    fs::write(another_dir_in_input.join("file3.txt"), "Content of file3").unwrap();

    // 2. Create file_list.txt with relative paths from the repository root
    //    The `pna` command will be run from the repo root (`/app`).
    //    TestResources.path() gives us something like `/app/target/tmp/pid/test_name/`
    //    We need to make file_list.txt paths relative to `/app`
    let file_list_path = base_path.join("file_list.txt");
    let mut file_list = fs::File::create(&file_list_path).unwrap();
    // Construct paths relative to where 'pna' will be run (repo root)
    // resources.path() is absolute. We need to make them relative to current_dir (which is /app for tests)
    // or, more robustly, ensure the paths written into file_list.txt are exactly what `pna create --files-from` expects.
    // The paths in file_list.txt are treated as relative to the current working directory OR absolute.
    // For simplicity and to match how users might use it, we'll make them relative to the input_dir's parent,
    // which is `base_path`. The `pna create` command itself won't have a CWD inside `base_path`.
    // Let's write paths relative to `base_path` for items inside `input_dir`.
    // No, the task asks for paths *from the input directory* into file_list.txt
    // "Write the relative paths of a subset of the files from the input directory into file_list.txt"
    // This means paths like "file1.txt" and "another_dir/file3.txt" if CWD was `input_dir`.
    // However, `pna create --files-from` takes paths relative to the CWD of `pna` itself, or absolute paths.
    // The most straightforward for testing is to use absolute paths in file_list.txt or paths relative to repo root.
    
    // Let's use paths relative to repo root, assuming test runs from /app
    // input_dir = /app/target/tmp/pid/test_name/in
    // file1.txt path for file_list.txt = target/tmp/pid/test_name/in/file1.txt

    let path_to_file1 = input_dir.join("file1.txt");
    let path_to_file3 = another_dir_in_input.join("file3.txt");

    // We need to get the path relative to /app. TestResources.path() is /app/target/tmp.../test_name
    // So, path_for_list = resources.path().strip_prefix("/app").unwrap().join("in").join("file1.txt") - this is complex.

    // Simpler: use absolute paths in file_list.txt. This is unambiguous.
    writeln!(file_list, "{}", path_to_file1.to_str().unwrap()).unwrap();
    writeln!(file_list, "{}", path_to_file3.to_str().unwrap()).unwrap();

    // 3. Define archive path and output directories
    let archive_path = base_path.join("archive_files_from.pna");
    let expected_out_dir = base_path.join("expected_out");
    let actual_out_dir = base_path.join("actual_out");
    fs::create_dir_all(&expected_out_dir).unwrap();
    fs::create_dir_all(&actual_out_dir).unwrap();

    // 4. Populate expected_out_dir
    // Since paths in file_list.txt are absolute, they will be stored as such.
    // We expect them to be extracted with their full paths.
    // To match the structure, expected_out needs to mirror where these files would land.
    // If /app/target/.../in/file1.txt is archived, it extracts to actual_out/app/target/.../in/file1.txt without strip.
    // This is probably not what the user wants. Let's assume files-from takes paths relative to CWD if not absolute.
    // If `pna create --files-from list.txt archive.pna .` (note the final dot for base path)
    // and list.txt contains `in/file1.txt`, it would archive `in/file1.txt` relative to CWD.
    // The problem statement implies paths in file_list.txt are relative to the input dir.
    // "Write the relative paths of a subset of the files from the input directory into file_list.txt"
    // This is tricky. If `file_list.txt` contains `in/file1.txt` and `in/another_dir/file3.txt`,
    // and `pna create ... --files-from file_list.txt archive.pna -C base_path_of_files_in_list`
    // The `-C` option changes the CWD for interpreting paths *in the file list*.
    // Let's re-evaluate. The original prompt:
    // "Write the relative paths of a subset of the files from the input directory into file_list.txt,
    // one path per line (e.g., create_files_from/in/file1.txt, create_files_from/in/another_dir/file3.txt)."
    // This example implies paths are relative to the project root if `create_files_from` is at project root.
    // Let's stick to this: paths in file_list.txt are relative to CWD of `pna` command.
    // CWD of `pna` is `/app`. `base_path` is `/app/target/tmp/.../create_files_from`.
    // So, `file_list.txt` should contain:
    // `target/tmp/.../create_files_from/in/file1.txt`
    // `target/tmp/.../create_files_from/in/another_dir/file3.txt`

    // Re-creating file_list.txt with paths relative to `/app`
    let current_dir = std::env::current_dir().unwrap(); // Should be /app
    let file_list_path = base_path.join("file_list.txt"); // e.g. /app/target/tmp/xxx/create_files_from/file_list.txt
    let mut file_list_writer = fs::File::create(&file_list_path).unwrap();

    let path_in_list1 = path_to_file1.strip_prefix(&current_dir).unwrap().to_str().unwrap();
    let path_in_list2 = path_to_file3.strip_prefix(&current_dir).unwrap().to_str().unwrap();
    writeln!(file_list_writer, "{}", path_in_list1).unwrap();
    writeln!(file_list_writer, "{}", path_in_list2).unwrap();
    
    // Populate expected_out_dir. These files will be extracted with these paths.
    // So, expected_out_dir will have target/tmp/.../create_files_from/in/file1.txt
    let expected_file1_path = expected_out_dir.join(path_in_list1);
    let expected_file3_path = expected_out_dir.join(path_in_list2);
    fs::create_dir_all(expected_file1_path.parent().unwrap()).unwrap();
    fs::copy(&path_to_file1, &expected_file1_path).unwrap();
    fs::create_dir_all(expected_file3_path.parent().unwrap()).unwrap();
    fs::copy(&path_to_file3, &expected_file3_path).unwrap();

    // 5. Run `pna create`
    let cli_args_create = vec![
        "pna",
        "create",
        archive_path.to_str().unwrap(),
        "--files-from",
        file_list_path.to_str().unwrap(),
        "--overwrite",
        "--unstable",
        "--quiet",
    ];
    let cli_create = cli::Cli::parse_from(cli_args_create);
    cli_create.exec().unwrap();

    // 6. Run `pna x` to extract. No stripping needed if paths are relative to repo root.
    // The files will be extracted to actual_out_dir/target/tmp/.../create_files_from/in/file1.txt
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
