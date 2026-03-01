use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{
    fs,
    path::{Path, PathBuf},
};

fn init_resource<P: AsRef<Path>>(dir: P) {
    let dir = dir.as_ref();
    if dir.exists() {
        fs::remove_dir_all(dir).unwrap();
    }
    fs::create_dir_all(dir).unwrap();

    fs::write(dir.join("text.txt"), b"content").unwrap();
    pna::fs::symlink(Path::new("text.txt"), dir.join("link.txt")).unwrap();

    fs::create_dir_all(dir.join("dir")).unwrap();
    fs::write(dir.join("dir/in_dir_text.txt"), b"dir_content").unwrap();
    pna::fs::symlink(
        Path::new("in_dir_text.txt"),
        dir.join("dir/in_dir_link.txt"),
    )
    .unwrap();
    pna::fs::symlink(Path::new("dir"), dir.join("link_dir")).unwrap();
}

fn init_broken_resource<P: AsRef<Path>>(dir: P) {
    let dir = dir.as_ref();
    if dir.exists() {
        fs::remove_dir_all(dir).unwrap();
    }
    fs::create_dir_all(dir).unwrap();

    // Create broken symlinks that point to non-existent targets
    pna::fs::symlink(Path::new("missing.txt"), dir.join("broken.txt")).unwrap();
    pna::fs::symlink(Path::new("missing_dir"), dir.join("broken_dir")).unwrap();
}

/// Precondition: source tree with regular files, dirs, and symlinks (all valid).
/// Action: run `pna create` without `--follow-links`, then run `pna extract`.
/// Expectation: symlinks are stored/extracted as links; regular files/dirs stay intact.
#[test]
fn symlink_no_follow() {
    setup();
    init_resource("symlink_no_follow/source");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "symlink_no_follow/symlink_no_follow.pna",
        "--overwrite",
        "--keep-dir",
        "symlink_no_follow/source",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry(
        "symlink_no_follow/symlink_no_follow.pna",
        |entry| match entry.header().path().as_str() {
            "symlink_no_follow/source" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "symlink_no_follow/source/text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "symlink_no_follow/source/dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "symlink_no_follow/source/dir/in_dir_text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "symlink_no_follow/source/dir/in_dir_link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
                assert_eq!(archive::read_symlink_target(&entry), "in_dir_text.txt");
            }
            "symlink_no_follow/source/link_dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
                assert_eq!(archive::read_symlink_target(&entry), "dir");
            }
            "symlink_no_follow/source/link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
                assert_eq!(archive::read_symlink_target(&entry), "text.txt");
            }
            path => unreachable!("unexpected entry found: {path}"),
        },
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "symlink_no_follow/symlink_no_follow.pna",
        "--overwrite",
        "--out-dir",
        "symlink_no_follow/dist",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(PathBuf::from("symlink_no_follow/dist/link.txt").is_symlink());
    assert!(PathBuf::from("symlink_no_follow/dist/link_dir").is_symlink());
    assert!(PathBuf::from("symlink_no_follow/dist/dir/in_dir_link.txt").is_symlink());
    assert_eq!(
        fs::read_to_string("symlink_no_follow/dist/dir/in_dir_text.txt").unwrap(),
        fs::read_to_string("symlink_no_follow/dist/link_dir/in_dir_text.txt").unwrap(),
    );
    assert_eq!(
        fs::read_link("symlink_no_follow/dist/link.txt").unwrap(),
        Path::new("text.txt"),
    );
    assert_eq!(
        fs::read_link("symlink_no_follow/dist/link_dir").unwrap(),
        Path::new("dir"),
    );
    assert_eq!(
        fs::read_link("symlink_no_follow/dist/dir/in_dir_link.txt").unwrap(),
        Path::new("in_dir_text.txt"),
    );
}

// FIXME: On GitHub Actions Windows runner disabled due to insufficient privileges for execution
/// Precondition: source tree with regular files, directories, and symlinks (all targets exist).
/// Action: run `pna create` with `--follow-links`, then run `pna extract`.
/// Expectation: links are resolved to their targets; archive/extract contains files/dirs, not symlinks.
#[cfg(unix)]
#[test]
fn symlink_follow() {
    setup();
    init_resource("symlink_follow/source");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "symlink_follow/symlink_follow.pna",
        "--overwrite",
        "--keep-dir",
        "--follow-links",
        "symlink_follow/source",
    ])
    .unwrap()
    .execute()
    .unwrap();
    archive::for_each_entry("symlink_follow/symlink_follow.pna", |entry| {
        match entry.header().path().as_str() {
            "symlink_follow/source" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "symlink_follow/source/text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "symlink_follow/source/dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "symlink_follow/source/dir/in_dir_text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "symlink_follow/source/dir/in_dir_link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "symlink_follow/source/link_dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "symlink_follow/source/link_dir/in_dir_link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "symlink_follow/source/link_dir/in_dir_text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "symlink_follow/source/link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            path => unreachable!("unexpected entry found: {path}"),
        }
    })
    .unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "symlink_follow/symlink_follow.pna",
        "--overwrite",
        "--out-dir",
        "symlink_follow/dist",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(!PathBuf::from("symlink_follow/dist/link.txt").is_symlink());
    assert!(!PathBuf::from("symlink_follow/dist/link_dir").is_symlink());
    assert!(!PathBuf::from("symlink_follow/dist/dir/in_dir_link.txt").is_symlink());
    assert_eq!(
        fs::read_to_string("symlink_follow/dist/dir/in_dir_link.txt").unwrap(),
        fs::read_to_string("symlink_follow/dist/dir/in_dir_text.txt").unwrap()
    );
    assert_eq!(
        fs::read_to_string("symlink_follow/dist/dir/in_dir_text.txt").unwrap(),
        fs::read_to_string("symlink_follow/dist/link_dir/in_dir_text.txt").unwrap(),
    );
}

/// Precondition: source tree contains broken file and dir symlinks.
/// Action: run `pna create` without `--follow-links`, then run `pna extract`.
/// Expectation: broken links are stored/extracted as symlinks; missing targets do not error.
#[test]
fn broken_symlink_no_follow() {
    setup();
    init_broken_resource("broken_symlink_no_follow/source");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "broken_symlink_no_follow/broken_symlink_no_follow.pna",
        "--overwrite",
        "--keep-dir",
        "broken_symlink_no_follow/source",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry(
        "broken_symlink_no_follow/broken_symlink_no_follow.pna",
        |entry| match entry.header().path().as_str() {
            "broken_symlink_no_follow/source" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "broken_symlink_no_follow/source/broken.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
                assert_eq!(archive::read_symlink_target(&entry), "missing.txt");
            }
            "broken_symlink_no_follow/source/broken_dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
                assert_eq!(archive::read_symlink_target(&entry), "missing_dir");
            }
            path => unreachable!("unexpected entry found: {path}"),
        },
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "broken_symlink_no_follow/broken_symlink_no_follow.pna",
        "--overwrite",
        "--out-dir",
        "broken_symlink_no_follow/dist",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(PathBuf::from("broken_symlink_no_follow/dist/broken.txt").is_symlink());
    assert!(PathBuf::from("broken_symlink_no_follow/dist/broken_dir").is_symlink());
    assert_eq!(
        fs::read_link("broken_symlink_no_follow/dist/broken.txt").unwrap(),
        Path::new("missing.txt"),
    );
    assert_eq!(
        fs::read_link("broken_symlink_no_follow/dist/broken_dir").unwrap(),
        Path::new("missing_dir"),
    );
}

// FIXME: On GitHub Actions Windows runner disabled due to insufficient privileges for execution
/// Precondition: source tree contains broken file and dir symlinks (targets missing).
/// Action: run `pna create` with `--follow-links`, then run `pna extract`.
/// Expectation: broken links stay symlinks (unresolvable); nothing is dropped or rewritten.
#[cfg(unix)]
#[test]
fn broken_symlink_follow() {
    setup();
    init_broken_resource("broken_symlink_follow/source");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "broken_symlink_follow/broken_symlink_follow.pna",
        "--overwrite",
        "--keep-dir",
        "--follow-links",
        "broken_symlink_follow/source",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry(
        "broken_symlink_follow/broken_symlink_follow.pna",
        |entry| match entry.header().path().as_str() {
            "broken_symlink_follow/source" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "broken_symlink_follow/source/broken.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
                assert_eq!(archive::read_symlink_target(&entry), "missing.txt");
            }
            "broken_symlink_follow/source/broken_dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
                assert_eq!(archive::read_symlink_target(&entry), "missing_dir");
            }
            path => unreachable!("unexpected entry found: {path}"),
        },
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "broken_symlink_follow/broken_symlink_follow.pna",
        "--overwrite",
        "--out-dir",
        "broken_symlink_follow/dist",
        "--strip-components",
        "2",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(PathBuf::from("broken_symlink_follow/dist/broken.txt").is_symlink());
    assert!(PathBuf::from("broken_symlink_follow/dist/broken_dir").is_symlink());
    assert_eq!(
        fs::read_link("broken_symlink_follow/dist/broken.txt").unwrap(),
        Path::new("missing.txt"),
    );
    assert_eq!(
        fs::read_link("broken_symlink_follow/dist/broken_dir").unwrap(),
        Path::new("missing_dir"),
    );
}

/// Precondition: top-level input is a file symlink (depth 0).
/// Action: run `pna create` without `--follow-links`, then extract.
/// Expectation: archive stores a symlink entry; extraction recreates the symlink path.
#[test]
fn symlink_depth0_no_follow_file() {
    // Ensure a symlink passed directly (depth 0) is stored as a symlink when not following links
    setup();
    let base = PathBuf::from("symlink_depth0_no_follow_file");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    fs::write(base.join("target.txt"), b"content").unwrap();
    pna::fs::symlink(Path::new("target.txt"), base.join("link.txt")).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "symlink_depth0_no_follow_file/symlink_depth0_no_follow_file.pna",
        "--overwrite",
        // pass the symlink itself (depth 0)
        "symlink_depth0_no_follow_file/link.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry(
        "symlink_depth0_no_follow_file/symlink_depth0_no_follow_file.pna",
        |entry| match entry.header().path().as_str() {
            "symlink_depth0_no_follow_file/link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
                assert_eq!(archive::read_symlink_target(&entry), "target.txt");
            }
            path => unreachable!("unexpected entry found: {path}"),
        },
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "symlink_depth0_no_follow_file/symlink_depth0_no_follow_file.pna",
        "--overwrite",
        "--out-dir",
        "symlink_depth0_no_follow_file/dist",
        "--strip-components",
        "1",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(PathBuf::from("symlink_depth0_no_follow_file/dist/link.txt").is_symlink());
    assert_eq!(
        fs::read_link("symlink_depth0_no_follow_file/dist/link.txt").unwrap(),
        Path::new("target.txt"),
    );
}

/// Precondition: top-level input is a directory symlink (depth 0) with `--no-keep-dir`.
/// Action: run `pna create` without `--follow-links`, then extract.
/// Expectation: directory symlink stored/extracted as symlink; target tree is not expanded.
#[test]
fn symlink_depth0_no_follow_dir() {
    // Ensure a directory symlink passed directly (depth 0) is stored as a symlink when not following links
    setup();
    let base = PathBuf::from("symlink_depth0_no_follow_dir");
    if base.exists() {
        fs::remove_dir_all(&base).unwrap();
    }
    fs::create_dir_all(&base).unwrap();

    fs::create_dir_all(base.join("dir")).unwrap();
    fs::write(base.join("dir/in_dir_text.txt"), b"dir_content").unwrap();
    pna::fs::symlink(Path::new("dir"), base.join("link_dir")).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "symlink_depth0_no_follow_dir/symlink_depth0_no_follow_dir.pna",
        "--overwrite",
        "--no-keep-dir",
        // pass the symlink itself (depth 0)
        "symlink_depth0_no_follow_dir/link_dir",
    ])
    .unwrap()
    .execute()
    .unwrap();

    archive::for_each_entry(
        "symlink_depth0_no_follow_dir/symlink_depth0_no_follow_dir.pna",
        |entry| match entry.header().path().as_str() {
            "symlink_depth0_no_follow_dir/link_dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
                assert_eq!(archive::read_symlink_target(&entry), "dir");
            }
            path => unreachable!("unexpected entry found: {path}"),
        },
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "symlink_depth0_no_follow_dir/symlink_depth0_no_follow_dir.pna",
        "--overwrite",
        "--out-dir",
        "symlink_depth0_no_follow_dir/dist",
        "--strip-components",
        "1",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(PathBuf::from("symlink_depth0_no_follow_dir/dist/link_dir").is_symlink());
    assert_eq!(
        fs::read_link("symlink_depth0_no_follow_dir/dist/link_dir").unwrap(),
        Path::new("dir"),
    );
}

/// Precondition: fixture with nested dir and file symlinks.
/// Action: run `pna create` with `-H`, then inspect entries.
/// Expectation: command-line symlink operands are resolved; nested symlinks remain links in the archive.
#[cfg(unix)]
#[test]
fn symlink_follow_command_line_partial() {
    setup();
    init_resource("symlink_follow_partial/source");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "symlink_follow_partial/partial.pna",
        "--overwrite",
        "--keep-dir",
        "-H",
        "symlink_follow_partial/source/dir",
        "symlink_follow_partial/source/link_dir",
        "symlink_follow_partial/source/link.txt",
        "symlink_follow_partial/source/text.txt",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();
    archive::for_each_entry("symlink_follow_partial/partial.pna", |entry| {
        match entry.header().path().as_str() {
            "symlink_follow_partial/source/dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory);
            }
            "symlink_follow_partial/source/dir/in_dir_link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
                assert_eq!(archive::read_symlink_target(&entry), "in_dir_text.txt");
            }
            "symlink_follow_partial/source/dir/in_dir_text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File);
            }
            "symlink_follow_partial/source/link_dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory);
            }
            "symlink_follow_partial/source/link_dir/in_dir_link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
                assert_eq!(archive::read_symlink_target(&entry), "in_dir_text.txt");
            }
            "symlink_follow_partial/source/link_dir/in_dir_text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File);
            }
            "symlink_follow_partial/source/link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File);
            }
            "symlink_follow_partial/source/text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File);
            }
            path => unreachable!("unexpected entry found: {path}"),
        }
    })
    .unwrap();
}
