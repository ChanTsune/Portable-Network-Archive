use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::Path;

/// Precondition: An archive contains files in nested directories and a manifest lists a subset.
/// Action: Create the archive via `pna experimental stdio --create`, then extract it using
///         `pna experimental stdio --extract --files-from <manifest>`.
/// Expectation: Only the manifest entries appear in the output tree; other archive members stay
///         untouched.
#[test]
fn stdio_extract_with_files_from() {
    setup();

    // Prepare input payload
    let base = Path::new("stdio_files_from");
    let input = base.join("in");
    fs::create_dir_all(&input).unwrap();
    fs::write(input.join("keep_a.txt"), "keep-a").unwrap();
    fs::write(input.join("keep_b.txt"), "keep-b").unwrap();
    fs::write(input.join("drop.txt"), "drop").unwrap();
    fs::create_dir_all(input.join("nested")).unwrap();
    fs::write(input.join("nested").join("keep_nested.txt"), "keep-nested").unwrap();

    // Create archive using stdio mode
    let archive_path = base.join("archive.pna");
    let mut create_cmd = cargo_bin_cmd!("pna");
    create_cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "--create",
        "--unstable",
        "--overwrite",
        "-f",
        archive_path.to_str().unwrap(),
        "-C",
        input.to_str().unwrap(),
        ".",
    ]);
    create_cmd.assert().success();

    // Prepare file list (newline-delimited)
    let list_path = base.join("files.txt");
    fs::create_dir_all(base).unwrap();
    fs::write(&list_path, "keep_a.txt\nnested/keep_nested.txt\n").unwrap();

    // Extract only files referenced by --files-from
    let output_dir = base.join("out");
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "--extract",
        "--unstable",
        "--files-from",
        list_path.to_str().unwrap(),
        "--out-dir",
        output_dir.to_str().unwrap(),
        "--overwrite",
        "-f",
        archive_path.to_str().unwrap(),
    ]);
    extract_cmd.assert().success();

    // Confirm only the requested files were extracted
    assert!(output_dir.join("keep_a.txt").exists());
    assert!(output_dir.join("nested").join("keep_nested.txt").exists());
    assert!(!output_dir.join("keep_b.txt").exists());
    assert!(!output_dir.join("drop.txt").exists());

    // Ensure extracted files match originals for the selected set
    assert_eq!(
        fs::read_to_string(input.join("keep_a.txt")).unwrap(),
        fs::read_to_string(output_dir.join("keep_a.txt")).unwrap()
    );
    assert_eq!(
        fs::read_to_string(input.join("nested").join("keep_nested.txt")).unwrap(),
        fs::read_to_string(output_dir.join("nested").join("keep_nested.txt")).unwrap()
    );
}
