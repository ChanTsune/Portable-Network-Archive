use crate::utils::{diff, setup, TestResources};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::{fs, io::Write, path::Path};

#[test]
fn create_split_archive() {
    setup();
    let resources = TestResources::extract_in("create_split_archive");
    let input_dir = resources.path_with_default("in");
    fs::create_dir_all(&input_dir).unwrap();

    // Create some files, one larger than the split size
    fs::write(input_dir.join("file1.txt"), "This is file 1.").unwrap();
    fs::write(input_dir.join("file2.txt"), "This is file 2, which is a bit longer.").unwrap();
    
    let mut large_file = fs::File::create(input_dir.join("large_file.dat")).unwrap();
    let mut buffer = Vec::with_capacity(1024);
    for i in 0..1024 {
        buffer.push((i % 256) as u8);
    }
    large_file.write_all(&buffer).unwrap(); // 1KB file

    let archive_base_name = "test_split_archive.pna";
    let archive_path = resources.path_with_default(archive_base_name);
    let extract_dir = resources.path_with_default("out");
    fs::create_dir_all(&extract_dir).unwrap();

    // Create split archive
    let cli_args_create = vec![
        "pna",
        "create",
        archive_path.to_str().unwrap(),
        input_dir.to_str().unwrap(),
        "--split",
        "512B", // Split size of 512 bytes
        "--overwrite",
        "--quiet", // Suppress progress output for cleaner test logs
    ];
    let cli_create = cli::Cli::parse_from(cli_args_create);
    cli_create.exec().unwrap();

    // Verify that multiple archive part files are created
    let part1_path = Path::new(&format!("{}.{}", archive_path.to_str().unwrap(), "001"));
    let part2_path = Path::new(&format!("{}.{}", archive_path.to_str().unwrap(), "002"));
    assert!(part1_path.exists(), "Archive part .001 should exist");
    assert!(part2_path.exists(), "Archive part .002 should exist (due to 1KB file and 512B split size)");

    // Extract split archive
    // Note: pna extract should automatically find other parts if .001 is specified,
    // or if the base name is specified (implementation dependent, testing with base name)
    let cli_args_extract = vec![
        "pna",
        "x", // Short alias for extract
        archive_path.to_str().unwrap(), // Provide base name for extraction
        "--out-dir",
        extract_dir.to_str().unwrap(),
        "--quiet",
    ];
    let cli_extract = cli::Cli::parse_from(cli_args_extract);
    cli_extract.exec().unwrap();

    // Compare the original input directory with the output directory
    let diff_output = diff(&input_dir, &extract_dir);
    assert!(diff_output.is_empty(), "Diff output should be empty, indicating identical directories. Diff:\n{}", diff_output);
}
