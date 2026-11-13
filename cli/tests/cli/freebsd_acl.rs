#![cfg(feature = "acl")]

use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use std::{fs, path::Path};

/// Precondition: The FreeBSD test directory resides on a native ACL-enabled filesystem.
/// Action: Archive and extract a file with its native ACL.
/// Expectation: Both the principal identity in the archive and the native ACL survive.
#[test]
fn native_acl_roundtrip() {
    setup();
    let root = Path::new("freebsd_native_acl");
    fs::create_dir_all(root).unwrap();
    let input = root.join("input.txt");
    fs::write(&input, b"FreeBSD ACL roundtrip\n").unwrap();

    let before = exacl::getfacl(&input, None).unwrap();
    let nfs4 = before
        .iter()
        .any(|entry| entry.kind == exacl::AclEntryKind::Everyone);
    if let Ok(expected) = std::env::var("PNA_TEST_FREEBSD_ACL_KIND") {
        assert_eq!(nfs4, expected == "nfsv4", "wrong ACL model in CI");
    }

    let archive = root.join("archive.pna");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "c",
            "-f",
            archive.to_str().unwrap(),
            "--overwrite",
            input.to_str().unwrap(),
            "--keep-acl",
            "--unstable",
        ])
        .assert()
        .success();

    let dump = cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "experimental",
            "acl",
            "get",
            "-f",
            archive.to_str().unwrap(),
            input.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let dump = String::from_utf8(dump).unwrap();
    let principal = if nfs4 { ":e::" } else { ":o::" };
    assert!(
        dump.lines().any(|line| line.starts_with(principal)),
        "expected native ACL principal {principal} in archive:\n{dump}"
    );

    let out = root.join("out");
    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "x",
            "-f",
            archive.to_str().unwrap(),
            "--overwrite",
            "--out-dir",
            out.to_str().unwrap(),
            "--keep-acl",
            "--unstable",
        ])
        .assert()
        .success();

    let extracted = out.join(&input);
    assert_eq!(fs::read(&extracted).unwrap(), fs::read(&input).unwrap());
    let after = exacl::getfacl(extracted, None).unwrap();
    assert_eq!(after, before, "native FreeBSD ACL must roundtrip exactly");
}
