use crate::utils::assert_archive_gid_gname;
use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::MetadataExt;

use crate::utils::TestEnv;

// --- Create Tests ---

#[test]
fn create_with_gid() {
    let env = TestEnv::new();
    env.create_file("file.txt", b"");
    let mut command = Command::cargo_bin("pna").unwrap();
    let assertion = command
        .arg("experimental")
        .arg("stdio")
        .arg("--create")
        .arg("--keep-permission")
        .arg("--gid")
        .arg("1234")
        .arg("-C")
        .arg(env.path())
        .arg("file.txt")
        .assert()
        .success();
    let archive = &assertion.get_output().stdout;
    assert_archive_gid_gname(archive, 1234, "1234");
}

#[test]
#[cfg(unix)]
fn create_with_gname() {
    let env = TestEnv::new();
    let meta = env.create_file("file.txt", b"");
    let mut command = Command::cargo_bin("pna").unwrap();
    let assertion = command
        .arg("experimental")
        .arg("stdio")
        .arg("--create")
        .arg("--keep-permission")
        .arg("--gname")
        .arg("testgroup")
        .arg("-C")
        .arg(env.path())
        .arg("file.txt")
        .assert()
        .success();
    let archive = &assertion.get_output().stdout;
    assert_archive_gid_gname(archive, meta.gid() as u64, "testgroup");
}

#[test]
fn create_with_gid_and_gname() {
    let env = TestEnv::new();
    env.create_file("file.txt", b"");
    let mut command = Command::cargo_bin("pna").unwrap();
    let assertion = command
        .arg("experimental")
        .arg("stdio")
        .arg("--create")
        .arg("--keep-permission")
        .arg("--gid")
        .arg("1234")
        .arg("--gname")
        .arg("testgroup")
        .arg("-C")
        .arg(env.path())
        .arg("file.txt")
        .assert()
        .success();
    let archive = &assertion.get_output().stdout;
    assert_archive_gid_gname(archive, 1234, "testgroup");
}

#[test]
fn create_with_group_name_and_id() {
    let env = TestEnv::new();
    env.create_file("file.txt", b"");
    let mut command = Command::cargo_bin("pna").unwrap();
    let assertion = command
        .arg("experimental")
        .arg("stdio")
        .arg("--create")
        .arg("--keep-permission")
        .arg("--group")
        .arg("testgroup:1234")
        .arg("-C")
        .arg(env.path())
        .arg("file.txt")
        .assert()
        .success();
    let archive = &assertion.get_output().stdout;
    assert_archive_gid_gname(archive, 1234, "testgroup");
}

// --- Extract Tests ---

#[test]
#[ignore = "requires root privileges to change ownership"]
#[cfg(unix)]
fn extract_with_gid_override() {
    let env = TestEnv::new();
    env.create_file("file.txt", b"content");
    let mut create_command = Command::cargo_bin("pna").unwrap();
    let create_assertion = create_command
        .arg("experimental")
        .arg("stdio")
        .arg("--create")
        .arg("--keep-permission")
        .arg("--gid")
        .arg("1000")
        .arg("--gname")
        .arg("group_a")
        .arg("-C")
        .arg(env.path())
        .arg("file.txt")
        .assert()
        .success();
    let archive_bytes = create_assertion.get_output().stdout.clone();
    let mut extract_command = Command::cargo_bin("pna").unwrap();
    extract_command
        .arg("experimental")
        .arg("stdio")
        .arg("--extract")
        .arg("--overwrite")
        .arg("--keep-permission")
        .arg("--gid")
        .arg("1234")
        .arg("-C")
        .arg(env.path())
        .write_stdin(archive_bytes)
        .assert()
        .success();
    let meta = fs::metadata(env.path().join("file.txt")).unwrap();
    assert_eq!(meta.gid(), 1234);
}

#[test]
#[ignore = "requires root privileges to change ownership"]
#[cfg(unix)]
fn extract_with_gname_gid_fallback() {
    let env = TestEnv::new();
    env.create_file("file.txt", b"content");
    let mut create_command = Command::cargo_bin("pna").unwrap();
    let create_assertion = create_command
        .arg("experimental")
        .arg("stdio")
        .arg("--create")
        .arg("--keep-permission")
        .arg("--gid")
        .arg("1000")
        .arg("-C")
        .arg(env.path())
        .arg("file.txt")
        .assert()
        .success();
    let archive_bytes = create_assertion.get_output().stdout.clone();
    let mut extract_command = Command::cargo_bin("pna").unwrap();
    extract_command
        .arg("experimental")
        .arg("stdio")
        .arg("--extract")
        .arg("--overwrite")
        .arg("--keep-permission")
        .arg("--gname")
        .arg("nonexistentgroup12345")
        .arg("--gid")
        .arg("1234")
        .arg("-C")
        .arg(env.path())
        .write_stdin(archive_bytes)
        .assert()
        .success();
    let meta = fs::metadata(env.path().join("file.txt")).unwrap();
    assert_eq!(meta.gid(), 1234);
}
