use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;

/// Precondition: A file with multiple hard links exists.
/// Action: Archive only one link with --check-links (-l).
/// Expectation: Warning is emitted about incomplete hardlink set.
#[test]
#[cfg(unix)]
fn stdio_with_check_links_incomplete_hardlinks() {
    setup();
    let _ = fs::remove_dir_all("stdio_check_links_warns");
    fs::create_dir_all("stdio_check_links_warns").unwrap();

    fs::write("stdio_check_links_warns/origin.txt", b"content").unwrap();
    fs::hard_link(
        "stdio_check_links_warns/origin.txt",
        "stdio_check_links_warns/link.txt",
    )
    .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "stdio",
            "--create",
            "-l",
            "stdio_check_links_warns/origin.txt",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "file has 2 links, only 1 archived",
        ));
}

/// Precondition: A file with multiple hard links exists.
/// Action: Archive all links with --check-links (-l).
/// Expectation: No warning is emitted since all links are archived.
#[test]
#[cfg(unix)]
fn stdio_with_check_links_complete_hardlinks() {
    setup();
    let _ = fs::remove_dir_all("stdio_check_links_complete");
    fs::create_dir_all("stdio_check_links_complete").unwrap();

    fs::write("stdio_check_links_complete/origin.txt", b"content").unwrap();
    fs::hard_link(
        "stdio_check_links_complete/origin.txt",
        "stdio_check_links_complete/link.txt",
    )
    .unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "stdio",
            "--create",
            "-l",
            "stdio_check_links_complete/origin.txt",
            "stdio_check_links_complete/link.txt",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("file has").not());
}

/// Precondition: A regular file with no additional hard links exists.
/// Action: Archive the file with --check-links (-l).
/// Expectation: No warning is emitted since the file has only one link.
#[test]
#[cfg(unix)]
fn stdio_with_check_links_single_link_file() {
    setup();
    let _ = fs::remove_dir_all("stdio_check_links_single");
    fs::create_dir_all("stdio_check_links_single").unwrap();

    fs::write("stdio_check_links_single/single.txt", b"content").unwrap();

    cargo_bin_cmd!("pna")
        .args([
            "experimental",
            "stdio",
            "--create",
            "-l",
            "stdio_check_links_single/single.txt",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("file has").not());
}
