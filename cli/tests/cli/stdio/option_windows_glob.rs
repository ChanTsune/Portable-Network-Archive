use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
#[cfg(windows)]
use predicates::prelude::PredicateBooleanExt;
#[cfg(windows)]
use std::collections::BTreeSet;
use std::{fs, path::Path};

#[cfg(windows)]
fn prepare_bsdtar_windows_glob_tree(base: &Path) -> std::path::PathBuf {
    let input = base.join("in");

    fs::create_dir_all(input.join("aaa").join("xxa")).unwrap();
    fs::create_dir_all(input.join("aaa").join("xxb")).unwrap();
    fs::create_dir_all(input.join("aaa").join("zzc")).unwrap();
    fs::write(input.join("aaa").join("file1"), "aaa-file1").unwrap();
    fs::write(input.join("aaa").join("xxa").join("file1"), "aaa-xxa-file1").unwrap();
    fs::write(input.join("aaa").join("xxb").join("file1"), "aaa-xxb-file1").unwrap();
    fs::write(input.join("aaa").join("zzc").join("file1"), "aaa-zzc-file1").unwrap();

    fs::create_dir_all(input.join("aab")).unwrap();
    fs::create_dir_all(input.join("aac")).unwrap();
    fs::create_dir_all(input.join("abb")).unwrap();
    fs::create_dir_all(input.join("abc")).unwrap();
    fs::create_dir_all(input.join("abd")).unwrap();

    fs::create_dir_all(input.join("bbb").join("xxa")).unwrap();
    fs::create_dir_all(input.join("bbb").join("xxb")).unwrap();
    fs::create_dir_all(input.join("bbb").join("zzc")).unwrap();
    fs::write(input.join("bbb").join("file1"), "bbb-file1").unwrap();
    fs::write(input.join("bbb").join("xxa").join("file1"), "bbb-xxa-file1").unwrap();
    fs::write(input.join("bbb").join("xxb").join("file1"), "bbb-xxb-file1").unwrap();
    fs::write(input.join("bbb").join("zzc").join("file1"), "bbb-zzc-file1").unwrap();

    fs::create_dir_all(input.join("bbc")).unwrap();
    fs::create_dir_all(input.join("bbd")).unwrap();
    fs::create_dir_all(input.join("bcc")).unwrap();
    fs::create_dir_all(input.join("bcd")).unwrap();
    fs::create_dir_all(input.join("bce")).unwrap();
    fs::create_dir_all(input.join("ccc")).unwrap();

    fs::create_dir_all(input.join("fff")).unwrap();
    fs::write(input.join("fff").join("aaaa"), "aaaa").unwrap();
    fs::write(input.join("fff").join("abba"), "abba").unwrap();
    fs::write(input.join("fff").join("abca"), "abca").unwrap();
    fs::write(input.join("fff").join("acba"), "acba").unwrap();
    fs::write(input.join("fff").join("acca"), "acca").unwrap();

    input
}

#[cfg(windows)]
fn create_stdio_archive(base: &Path, patterns: &[&str]) -> std::path::PathBuf {
    let input = prepare_bsdtar_windows_glob_tree(base);
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
    ]);
    create_cmd.args(patterns);
    create_cmd.assert().success();

    archive_path
}

#[cfg(windows)]
fn list_stdio_archive(archive_path: &Path) -> BTreeSet<String> {
    let mut list_cmd = cargo_bin_cmd!("pna");
    let output = list_cmd
        .args([
            "--quiet",
            "experimental",
            "stdio",
            "--list",
            "-f",
            archive_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    std::str::from_utf8(&output)
        .unwrap()
        .lines()
        .map(|line| line.to_string())
        .collect()
}

#[cfg(windows)]
fn assert_windows_glob_archive(base_name: &str, patterns: &[&str], expected: &[&str]) {
    setup();
    let base = Path::new(base_name);
    let archive_path = create_stdio_archive(base, patterns);
    let actual = list_stdio_archive(&archive_path);
    let expected = expected.iter().map(|entry| entry.to_string()).collect();
    assert_eq!(actual, expected);
}

#[cfg(unix)]
#[test]
fn stdio_create_does_not_expand_wildcards_on_unix() {
    setup();

    let base = Path::new("stdio_windows_glob_unix");
    let input = base.join("in");
    fs::create_dir_all(input.join("aaa")).unwrap();
    fs::create_dir_all(input.join("aab")).unwrap();
    fs::write(input.join("aaa").join("file1.txt"), "aaa").unwrap();
    fs::write(input.join("aab").join("file1.txt"), "aab").unwrap();

    let archive_path = base.join("archive.pna");
    let mut cmd = cargo_bin_cmd!("pna");
    cmd.args([
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
        "a*",
    ]);
    cmd.assert().failure();
    assert!(!archive_path.exists());
}

#[cfg(windows)]
#[test]
fn stdio_create_expands_wildcards_on_windows() {
    assert_windows_glob_archive(
        "stdio_windows_glob_windows",
        &["a*"],
        &[
            "aaa/",
            "aaa/file1",
            "aaa/xxa/",
            "aaa/xxa/file1",
            "aaa/xxb/",
            "aaa/xxb/file1",
            "aaa/zzc/",
            "aaa/zzc/file1",
            "aab/",
            "aac/",
            "abb/",
            "abc/",
            "abd/",
        ],
    );
}

#[cfg(windows)]
#[test]
fn stdio_create_expands_question_mark_windows_glob() {
    assert_windows_glob_archive(
        "stdio_windows_glob_question",
        &["??c"],
        &["aac/", "abc/", "bbc/", "bcc/", "ccc/"],
    );
}

#[cfg(windows)]
#[test]
fn stdio_create_expands_suffix_windows_glob() {
    assert_windows_glob_archive(
        "stdio_windows_glob_suffix",
        &["*c"],
        &["aac/", "abc/", "bbc/", "bcc/", "ccc/"],
    );
}

#[cfg(windows)]
#[test]
fn stdio_create_expands_nested_forward_slash_windows_glob() {
    assert_windows_glob_archive(
        "stdio_windows_glob_nested_forward",
        &["fff/a?ca"],
        &["fff/abca", "fff/acca"],
    );
}

#[cfg(windows)]
#[test]
fn stdio_create_expands_backslash_directory_glob() {
    assert_windows_glob_archive(
        "stdio_windows_glob_backslash",
        &[r"aaa\*"],
        &[
            "aaa/file1",
            "aaa/xxa/",
            "aaa/xxa/file1",
            "aaa/xxb/",
            "aaa/xxb/file1",
            "aaa/zzc/",
            "aaa/zzc/file1",
        ],
    );
}

#[cfg(windows)]
#[test]
fn stdio_create_expands_multiple_windows_globs() {
    assert_windows_glob_archive(
        "stdio_windows_glob_multiple",
        &[r"fff\a?ca", r"aaa\xx*"],
        &[
            "aaa/xxa/",
            "aaa/xxa/file1",
            "aaa/xxb/",
            "aaa/xxb/file1",
            "fff/abca",
            "fff/acca",
        ],
    );
}

#[cfg(windows)]
#[test]
fn stdio_create_backslash_glob_keeps_unmatched_entries_out() {
    setup();

    let base = Path::new("stdio_windows_glob_backslash_filter");
    let input = base.join("in");
    fs::create_dir_all(input.join("aaa").join("xxa")).unwrap();
    fs::create_dir_all(input.join("aaa").join("xxb")).unwrap();
    fs::create_dir_all(input.join("aaa").join("zzc")).unwrap();
    fs::write(input.join("aaa").join("xxa").join("file1.txt"), "xxa").unwrap();
    fs::write(input.join("aaa").join("xxb").join("file1.txt"), "xxb").unwrap();
    fs::write(input.join("aaa").join("zzc").join("file1.txt"), "zzc").unwrap();

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
        r"aaa\xx*",
    ]);
    create_cmd.assert().success();

    let mut list_cmd = cargo_bin_cmd!("pna");
    list_cmd.args([
        "--quiet",
        "experimental",
        "stdio",
        "--list",
        "-f",
        archive_path.to_str().unwrap(),
    ]);
    list_cmd
        .assert()
        .success()
        .stdout(predicates::str::contains("aaa/xxa/"))
        .stdout(predicates::str::contains("aaa/xxa/file1.txt"))
        .stdout(predicates::str::contains("aaa/xxb/"))
        .stdout(predicates::str::contains("aaa/xxb/file1.txt"))
        .stdout(predicates::str::contains("aaa/zzc/").not());
}
