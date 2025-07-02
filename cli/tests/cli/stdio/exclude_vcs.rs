use crate::utils::{diff::diff, setup, TestResources};
use assert_cmd::Command as Cmd;
use std::fs;
use std::path::Path;

#[test]
fn stdio_with_exclude_vcs() {
    // Setup test environment and extract input files
    setup();
    TestResources::extract_in("raw/", "stdio_with_exclude_vcs/in/").unwrap();
    // Create VCS files
    let vcs_files = [
        "stdio_with_exclude_vcs/in/raw/.git/HEAD",
        "stdio_with_exclude_vcs/in/raw/.git/config",
        "stdio_with_exclude_vcs/in/raw/.gitignore",
        "stdio_with_exclude_vcs/in/raw/.svn/entries",
        "stdio_with_exclude_vcs/in/raw/.hg/hgrc",
        "stdio_with_exclude_vcs/in/raw/.hgignore",
        "stdio_with_exclude_vcs/in/raw/.bzr/branch-format",
        "stdio_with_exclude_vcs/in/raw/.bzrignore",
        "stdio_with_exclude_vcs/in/raw/CVS/Root",
        "stdio_with_exclude_vcs/in/raw/.gitmodules",
        "stdio_with_exclude_vcs/in/raw/.gitattributes",
    ];
    for file in vcs_files.iter() {
        if let Some(parent) = Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }
    // Create regular files
    let regular_files = [
        "stdio_with_exclude_vcs/in/raw/regular.txt",
        "stdio_with_exclude_vcs/in/raw/data.csv",
        "stdio_with_exclude_vcs/in/raw/document.pdf",
    ];
    for file in regular_files.iter() {
        fs::write(file, "regular file content").unwrap();
    }
    // Create archive to stdout via stdio (with --exclude-vcs)
    let mut create_cmd = Cmd::cargo_bin("pna").unwrap();
    create_cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "--create",
        "--overwrite",
        "--unstable",
        "--exclude-vcs",
        "stdio_with_exclude_vcs/in/raw/regular.txt",
        "stdio_with_exclude_vcs/in/raw/data.csv",
        "stdio_with_exclude_vcs/in/raw/document.pdf",
    ]);
    let archive_data = create_cmd.assert().get_output().stdout.clone();
    // Extract archive from stdin via stdio (with --exclude-vcs)
    let out_dir = "stdio_with_exclude_vcs/out/";
    let mut extract_cmd = Cmd::cargo_bin("pna").unwrap();
    extract_cmd.write_stdin(archive_data).args([
        "--quiet",
        "experimental",
        "stdio",
        "--extract",
        "--overwrite",
        "--unstable",
        "--exclude-vcs",
        "--out-dir",
        out_dir,
    ]);
    extract_cmd.assert().success();
    // Remove VCS files from input for fair comparison
    for file in vcs_files.iter() {
        let _ = fs::remove_file(file);
    }
    // Compare input and output directories to ensure VCS files are excluded
    diff("stdio_with_exclude_vcs/in/raw/", format!("{out_dir}/raw/")).unwrap();
}

#[test]
fn stdio_without_exclude_vcs() {
    // Setup test environment and extract input files
    setup();
    TestResources::extract_in("raw/", "stdio_without_exclude_vcs/in/").unwrap();
    // Create VCS files
    let vcs_files = [
        "stdio_without_exclude_vcs/in/raw/.git/HEAD",
        "stdio_without_exclude_vcs/in/raw/.git/config",
        "stdio_without_exclude_vcs/in/raw/.gitignore",
        "stdio_without_exclude_vcs/in/raw/.svn/entries",
        "stdio_without_exclude_vcs/in/raw/.hg/hgrc",
        "stdio_without_exclude_vcs/in/raw/.hgignore",
        "stdio_without_exclude_vcs/in/raw/.bzr/branch-format",
        "stdio_without_exclude_vcs/in/raw/.bzrignore",
        "stdio_without_exclude_vcs/in/raw/CVS/Root",
        "stdio_without_exclude_vcs/in/raw/.gitmodules",
        "stdio_without_exclude_vcs/in/raw/.gitattributes",
    ];
    for file in vcs_files.iter() {
        if let Some(parent) = Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }
    // Create regular files
    let regular_files = [
        "stdio_without_exclude_vcs/in/raw/regular.txt",
        "stdio_without_exclude_vcs/in/raw/data.csv",
        "stdio_without_exclude_vcs/in/raw/document.pdf",
    ];
    for file in regular_files.iter() {
        fs::write(file, "regular file content").unwrap();
    }
    // Create archive to stdout via stdio (without --exclude-vcs)
    let mut create_cmd = Cmd::cargo_bin("pna").unwrap();
    create_cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "--create",
        "--overwrite",
        "--unstable",
        "stdio_without_exclude_vcs/in/raw/regular.txt",
        "stdio_without_exclude_vcs/in/raw/data.csv",
        "stdio_without_exclude_vcs/in/raw/document.pdf",
    ]);
    let archive_data = create_cmd.assert().get_output().stdout.clone();
    // Extract archive from stdin via stdio (without --exclude-vcs)
    let out_dir = "stdio_without_exclude_vcs/out/";
    let mut extract_cmd = Cmd::cargo_bin("pna").unwrap();
    extract_cmd.write_stdin(archive_data).args([
        "--quiet",
        "experimental",
        "stdio",
        "--extract",
        "--overwrite",
        "--unstable",
        "--out-dir",
        out_dir,
    ]);
    extract_cmd.assert().success();
    // Compare input and output directories to ensure VCS files are included
    diff(
        "stdio_without_exclude_vcs/in/raw/",
        format!("{out_dir}/raw/"),
    )
    .unwrap();
}
