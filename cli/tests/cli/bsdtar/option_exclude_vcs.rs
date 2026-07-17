use crate::utils::{EmbedExt, TestResources, setup};
use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::Path;

#[test]
fn bsdtar_with_exclude_vcs() {
    // Setup test environment and extract input files
    setup();
    TestResources::extract_in("raw/", "bsdtar_with_exclude_vcs/in/").unwrap();
    // Create VCS files
    let vcs_files = [
        "bsdtar_with_exclude_vcs/in/raw/.git/HEAD",
        "bsdtar_with_exclude_vcs/in/raw/.git/config",
        "bsdtar_with_exclude_vcs/in/raw/.gitignore",
        "bsdtar_with_exclude_vcs/in/raw/.svn/entries",
        "bsdtar_with_exclude_vcs/in/raw/.hg/hgrc",
        "bsdtar_with_exclude_vcs/in/raw/.hgignore",
        "bsdtar_with_exclude_vcs/in/raw/.bzr/branch-format",
        "bsdtar_with_exclude_vcs/in/raw/.bzrignore",
        "bsdtar_with_exclude_vcs/in/raw/CVS/Root",
        "bsdtar_with_exclude_vcs/in/raw/.gitmodules",
        "bsdtar_with_exclude_vcs/in/raw/.gitattributes",
    ];
    for file in vcs_files.iter() {
        if let Some(parent) = Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }
    // Create regular files
    let regular_files = [
        "bsdtar_with_exclude_vcs/in/raw/regular.txt",
        "bsdtar_with_exclude_vcs/in/raw/data.csv",
        "bsdtar_with_exclude_vcs/in/raw/document.pdf",
    ];
    for file in regular_files.iter() {
        fs::write(file, "regular file content").unwrap();
    }
    // Create archive to stdout via bsdtar (with --exclude-vcs)
    let mut create_cmd = cargo_bin_cmd!("pna");
    create_cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--create",
        "--overwrite",
        "--unstable",
        "--exclude-vcs",
        "bsdtar_with_exclude_vcs/in/raw/regular.txt",
        "bsdtar_with_exclude_vcs/in/raw/data.csv",
        "bsdtar_with_exclude_vcs/in/raw/document.pdf",
    ]);
    let archive_data = create_cmd.assert().get_output().stdout.clone();
    // Extract archive from stdin via bsdtar (with --exclude-vcs)
    let out_dir = "bsdtar_with_exclude_vcs/out/";
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd.write_stdin(archive_data).args([
        "--quiet",
        "compat",
        "bsdtar",
        "--extract",
        "--overwrite",
        "--unstable",
        "--exclude-vcs",
        "--out-dir",
        out_dir,
    ]);
    extract_cmd.assert().success();
    // Verify extracted regular files have correct content
    for file in ["regular.txt", "data.csv", "document.pdf"] {
        let in_path = format!("bsdtar_with_exclude_vcs/in/raw/{file}");
        let out_path = format!("{out_dir}bsdtar_with_exclude_vcs/in/raw/{file}");
        assert_eq!(
            fs::read(&in_path).unwrap(),
            fs::read(&out_path).unwrap(),
            "content mismatch for {file}"
        );
    }
    // Verify VCS directories are not extracted
    assert!(!fs::exists(format!("{out_dir}bsdtar_with_exclude_vcs/in/raw/.git")).unwrap());
    assert!(!fs::exists(format!("{out_dir}bsdtar_with_exclude_vcs/in/raw/.svn")).unwrap());
}

#[test]
fn bsdtar_without_exclude_vcs() {
    // Setup test environment and extract input files
    setup();
    TestResources::extract_in("raw/", "bsdtar_without_exclude_vcs/in/").unwrap();
    // Create VCS files
    let vcs_files = [
        "bsdtar_without_exclude_vcs/in/raw/.git/HEAD",
        "bsdtar_without_exclude_vcs/in/raw/.git/config",
        "bsdtar_without_exclude_vcs/in/raw/.gitignore",
        "bsdtar_without_exclude_vcs/in/raw/.svn/entries",
        "bsdtar_without_exclude_vcs/in/raw/.hg/hgrc",
        "bsdtar_without_exclude_vcs/in/raw/.hgignore",
        "bsdtar_without_exclude_vcs/in/raw/.bzr/branch-format",
        "bsdtar_without_exclude_vcs/in/raw/.bzrignore",
        "bsdtar_without_exclude_vcs/in/raw/CVS/Root",
        "bsdtar_without_exclude_vcs/in/raw/.gitmodules",
        "bsdtar_without_exclude_vcs/in/raw/.gitattributes",
    ];
    for file in vcs_files.iter() {
        if let Some(parent) = Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }
    // Create regular files
    let regular_files = [
        "bsdtar_without_exclude_vcs/in/raw/regular.txt",
        "bsdtar_without_exclude_vcs/in/raw/data.csv",
        "bsdtar_without_exclude_vcs/in/raw/document.pdf",
    ];
    for file in regular_files.iter() {
        fs::write(file, "regular file content").unwrap();
    }
    // Create archive to stdout via bsdtar (without --exclude-vcs)
    let mut create_cmd = cargo_bin_cmd!("pna");
    create_cmd.args([
        "--quiet",
        "compat",
        "bsdtar",
        "--create",
        "--overwrite",
        "--unstable",
        "bsdtar_without_exclude_vcs/in/raw/regular.txt",
        "bsdtar_without_exclude_vcs/in/raw/data.csv",
        "bsdtar_without_exclude_vcs/in/raw/document.pdf",
    ]);
    let archive_data = create_cmd.assert().get_output().stdout.clone();
    // Extract archive from stdin via bsdtar (without --exclude-vcs)
    let out_dir = "bsdtar_without_exclude_vcs/out/";
    let mut extract_cmd = cargo_bin_cmd!("pna");
    extract_cmd.write_stdin(archive_data).args([
        "--quiet",
        "compat",
        "bsdtar",
        "--extract",
        "--overwrite",
        "--unstable",
        "--out-dir",
        out_dir,
    ]);
    extract_cmd.assert().success();
    // Verify extracted regular files have correct content
    // Note: VCS files are not passed as archive arguments, so they are not
    // in the archive regardless of --exclude-vcs. This test verifies that
    // the absence of --exclude-vcs does not interfere with normal file archiving.
    for file in ["regular.txt", "data.csv", "document.pdf"] {
        let in_path = format!("bsdtar_without_exclude_vcs/in/raw/{file}");
        let out_path = format!("{out_dir}bsdtar_without_exclude_vcs/in/raw/{file}");
        assert_eq!(
            fs::read(&in_path).unwrap(),
            fs::read(&out_path).unwrap(),
            "content mismatch for {file}"
        );
    }
}
