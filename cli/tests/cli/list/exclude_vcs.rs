use crate::utils::{setup, EmbedExt, TestResources};
use assert_cmd::Command as Cmd;
use std::fs;

#[test]
fn list_with_exclude_vcs() {
    setup();
    TestResources::extract_in("raw/", "list_with_exclude_vcs/in/").unwrap();

    let vcs_files = [
        "list_with_exclude_vcs/in/raw/.git/HEAD",
        "list_with_exclude_vcs/in/raw/.git/config",
        "list_with_exclude_vcs/in/raw/.gitignore",
        "list_with_exclude_vcs/in/raw/.svn/entries",
        "list_with_exclude_vcs/in/raw/.hg/hgrc",
        "list_with_exclude_vcs/in/raw/.hgignore",
        "list_with_exclude_vcs/in/raw/.bzr/branch-format",
        "list_with_exclude_vcs/in/raw/.bzrignore",
        "list_with_exclude_vcs/in/raw/CVS/Root",
        "list_with_exclude_vcs/in/raw/.gitmodules",
        "list_with_exclude_vcs/in/raw/.gitattributes",
    ];
    for file in vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }

    let regular_files = [
        "list_with_exclude_vcs/in/raw/regular.txt",
        "list_with_exclude_vcs/in/raw/data.csv",
        "list_with_exclude_vcs/in/raw/document.pdf",
        "list_with_exclude_vcs/in/raw/empty.txt",
        "list_with_exclude_vcs/in/raw/first/second/third/pna.txt",
        "list_with_exclude_vcs/in/raw/images/icon.bmp",
        "list_with_exclude_vcs/in/raw/images/icon.png",
        "list_with_exclude_vcs/in/raw/images/icon.svg",
        "list_with_exclude_vcs/in/raw/parent/child.txt",
        "list_with_exclude_vcs/in/raw/pna/empty.pna",
        "list_with_exclude_vcs/in/raw/pna/nest.pna",
        "list_with_exclude_vcs/in/raw/text.txt",
    ];
    for file in regular_files {
        fs::write(file, "regular file content").unwrap();
    }

    // Create archive
    let mut cmd = Cmd::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        "list_with_exclude_vcs/list_with_exclude_vcs.pna",
        "--overwrite",
        "list_with_exclude_vcs/in/",
    ])
    .assert()
    .success();

    // Sort entries for stable order
    let mut cmd = Cmd::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "experimental",
        "sort",
        "-f",
        "list_with_exclude_vcs/list_with_exclude_vcs.pna",
    ])
    .assert()
    .success();

    // Test list with --exclude-vcs
    let mut cmd = Cmd::cargo_bin("pna").unwrap();
    let assert = cmd
        .args([
            "list",
            "list_with_exclude_vcs/list_with_exclude_vcs.pna",
            "--exclude-vcs",
        ])
        .assert();

    // Confirm that only regular files are output, and VCS files are excluded
    assert.stdout(concat!(
        "list_with_exclude_vcs/in/raw/data.csv\n",
        "list_with_exclude_vcs/in/raw/document.pdf\n",
        "list_with_exclude_vcs/in/raw/empty.txt\n",
        "list_with_exclude_vcs/in/raw/first/second/third/pna.txt\n",
        "list_with_exclude_vcs/in/raw/images/icon.bmp\n",
        "list_with_exclude_vcs/in/raw/images/icon.png\n",
        "list_with_exclude_vcs/in/raw/images/icon.svg\n",
        "list_with_exclude_vcs/in/raw/parent/child.txt\n",
        "list_with_exclude_vcs/in/raw/pna/empty.pna\n",
        "list_with_exclude_vcs/in/raw/pna/nest.pna\n",
        "list_with_exclude_vcs/in/raw/regular.txt\n",
        "list_with_exclude_vcs/in/raw/text.txt\n",
    ));
}

#[test]
fn list_without_exclude_vcs() {
    setup();
    TestResources::extract_in("raw/", "list_without_exclude_vcs/in/").unwrap();

    let vcs_files = [
        "list_without_exclude_vcs/in/raw/.git/HEAD",
        "list_without_exclude_vcs/in/raw/.git/config",
        "list_without_exclude_vcs/in/raw/.gitignore",
        "list_without_exclude_vcs/in/raw/.svn/entries",
        "list_without_exclude_vcs/in/raw/.hg/hgrc",
        "list_without_exclude_vcs/in/raw/.hgignore",
        "list_without_exclude_vcs/in/raw/.bzr/branch-format",
        "list_without_exclude_vcs/in/raw/.bzrignore",
        "list_without_exclude_vcs/in/raw/CVS/Root",
        "list_without_exclude_vcs/in/raw/.gitmodules",
        "list_without_exclude_vcs/in/raw/.gitattributes",
    ];
    for file in vcs_files {
        if let Some(parent) = std::path::Path::new(file).parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file, "vcs file content").unwrap();
    }

    let regular_files = [
        "list_without_exclude_vcs/in/raw/regular.txt",
        "list_without_exclude_vcs/in/raw/data.csv",
        "list_without_exclude_vcs/in/raw/document.pdf",
        "list_without_exclude_vcs/in/raw/empty.txt",
        "list_without_exclude_vcs/in/raw/first/second/third/pna.txt",
        "list_without_exclude_vcs/in/raw/images/icon.bmp",
        "list_without_exclude_vcs/in/raw/images/icon.png",
        "list_without_exclude_vcs/in/raw/images/icon.svg",
        "list_without_exclude_vcs/in/raw/parent/child.txt",
        "list_without_exclude_vcs/in/raw/pna/empty.pna",
        "list_without_exclude_vcs/in/raw/pna/nest.pna",
        "list_without_exclude_vcs/in/raw/text.txt",
    ];
    for file in regular_files {
        fs::write(file, "regular file content").unwrap();
    }

    // Create archive
    let mut cmd = Cmd::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "c",
        "list_without_exclude_vcs/list_without_exclude_vcs.pna",
        "--overwrite",
        "list_without_exclude_vcs/in/",
    ])
    .assert()
    .success();

    // Sort entries for stable order
    let mut cmd = Cmd::cargo_bin("pna").unwrap();
    cmd.args([
        "--quiet",
        "experimental",
        "sort",
        "-f",
        "list_without_exclude_vcs/list_without_exclude_vcs.pna",
    ])
    .assert()
    .success();

    // Test list without --exclude-vcs
    let mut cmd = Cmd::cargo_bin("pna").unwrap();
    let assert = cmd
        .args([
            "list",
            "list_without_exclude_vcs/list_without_exclude_vcs.pna",
        ])
        .assert();

    // Confirm that all files, including VCS files, are output
    assert.stdout(concat!(
        "list_without_exclude_vcs/in/raw/.bzr/branch-format\n",
        "list_without_exclude_vcs/in/raw/.bzrignore\n",
        "list_without_exclude_vcs/in/raw/.git/HEAD\n",
        "list_without_exclude_vcs/in/raw/.git/config\n",
        "list_without_exclude_vcs/in/raw/.gitattributes\n",
        "list_without_exclude_vcs/in/raw/.gitignore\n",
        "list_without_exclude_vcs/in/raw/.gitmodules\n",
        "list_without_exclude_vcs/in/raw/.hg/hgrc\n",
        "list_without_exclude_vcs/in/raw/.hgignore\n",
        "list_without_exclude_vcs/in/raw/.svn/entries\n",
        "list_without_exclude_vcs/in/raw/CVS/Root\n",
        "list_without_exclude_vcs/in/raw/data.csv\n",
        "list_without_exclude_vcs/in/raw/document.pdf\n",
        "list_without_exclude_vcs/in/raw/empty.txt\n",
        "list_without_exclude_vcs/in/raw/first/second/third/pna.txt\n",
        "list_without_exclude_vcs/in/raw/images/icon.bmp\n",
        "list_without_exclude_vcs/in/raw/images/icon.png\n",
        "list_without_exclude_vcs/in/raw/images/icon.svg\n",
        "list_without_exclude_vcs/in/raw/parent/child.txt\n",
        "list_without_exclude_vcs/in/raw/pna/empty.pna\n",
        "list_without_exclude_vcs/in/raw/pna/nest.pna\n",
        "list_without_exclude_vcs/in/raw/regular.txt\n",
        "list_without_exclude_vcs/in/raw/text.txt\n",
    ));
}
