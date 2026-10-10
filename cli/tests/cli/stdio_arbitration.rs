use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

#[test]
fn native_secondary_stdin_requires_a_file_archive() {
    let cases: &[&[&str]] = &[
        &["append", "--unstable", "--files-from-stdin"],
        &["update", "--unstable", "--files-from-stdin"],
        &["delete", "--unstable", "--files-from-stdin"],
        &["experimental", "acl", "set", "--restore-from-stdin"],
        &["xattr", "set", "--restore-from-stdin"],
        &["xattr", "set", "--restore", "-"],
    ];

    for args in cases {
        cargo_bin_cmd!("pna")
            .args(*args)
            .assert()
            .failure()
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("--file <ARCHIVE>"));
    }
}

#[test]
fn bsdtar_append_rejects_stdin_for_base_and_included_archive() {
    cargo_bin_cmd!("pna")
        .args(["compat", "bsdtar", "--append", "@-"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "standard input cannot be used for both archive input and archive source (@ or @-); specify the archive with --file",
        ));
}
