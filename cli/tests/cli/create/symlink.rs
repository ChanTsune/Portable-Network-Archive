use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::cli;
use std::{
    collections::HashMap,
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
    pna::fs::symlink(Path::new("missing_target"), dir.join("broken_link")).unwrap();
}

/// Precondition: The source tree contains regular files, directories, and symlinks (all valid).
/// Action: Run `pna create` without `--follow-links`, then run `pna extract`.
/// Expectation: Symlinks are stored and extracted as links; regular files and directories stay intact.
#[test]
fn symlink_no_follow() {
    setup();
    init_resource("symlink_no_follow/source");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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
        "-f",
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
/// Precondition: The source tree contains regular files, directories, and symlinks (all targets exist).
/// Action: Run `pna create` with `--follow-links`, then run `pna extract`.
/// Expectation: Links are resolved to their targets; the archive and extraction contain files and directories, not symlinks.
#[cfg(unix)]
#[test]
fn symlink_follow() {
    setup();
    init_resource("symlink_follow/source");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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
        "-f",
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

/// Precondition: The source tree contains broken file and directory symlinks.
/// Action: Run `pna create` without `--follow-links`, then run `pna extract`.
/// Expectation: Broken links are stored and extracted as symlinks; missing targets do not cause errors.
#[test]
fn broken_symlink_no_follow() {
    setup();
    init_broken_resource("broken_symlink_no_follow/source");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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
            "broken_symlink_no_follow/source/broken_link" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
                assert_eq!(archive::read_symlink_target(&entry), "missing_target");
            }
            path => unreachable!("unexpected entry found: {path}"),
        },
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
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
    assert!(PathBuf::from("broken_symlink_no_follow/dist/broken_link").is_symlink());
    assert_eq!(
        fs::read_link("broken_symlink_no_follow/dist/broken.txt").unwrap(),
        Path::new("missing.txt"),
    );
    assert_eq!(
        fs::read_link("broken_symlink_no_follow/dist/broken_link").unwrap(),
        Path::new("missing_target"),
    );
}

// FIXME: On GitHub Actions Windows runner disabled due to insufficient privileges for execution
/// Precondition: The source tree contains broken file and directory symlinks (targets missing).
/// Action: Run `pna create` with `--follow-links`, then run `pna extract`.
/// Expectation: Broken links stay as symlinks (unresolvable); nothing is dropped or rewritten.
#[cfg(unix)]
#[test]
fn broken_symlink_follow() {
    setup();
    init_broken_resource("broken_symlink_follow/source");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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
            "broken_symlink_follow/source/broken_link" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
                assert_eq!(archive::read_symlink_target(&entry), "missing_target");
            }
            path => unreachable!("unexpected entry found: {path}"),
        },
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
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
    assert!(PathBuf::from("broken_symlink_follow/dist/broken_link").is_symlink());
    assert_eq!(
        fs::read_link("broken_symlink_follow/dist/broken.txt").unwrap(),
        Path::new("missing.txt"),
    );
    assert_eq!(
        fs::read_link("broken_symlink_follow/dist/broken_link").unwrap(),
        Path::new("missing_target"),
    );
}

/// Precondition: The top-level input argument is a file symlink.
/// Action: Run `pna create` without `--follow-links`, then run `pna extract`.
/// Expectation: The archive stores a symlink entry; extraction recreates the symlink.
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
        "-f",
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
        "-f",
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

/// Precondition: The top-level input argument is a directory symlink with `--no-keep-dir`.
/// Action: Run `pna create` without `--follow-links`, then run `pna extract`.
/// Expectation: The directory symlink is stored and extracted as a symlink; the target tree is not expanded.
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
        "-f",
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
        "-f",
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

/// Precondition: The source tree contains nested directory and file symlinks.
/// Action: Run `pna create` with `-H`, then inspect the archive entries.
/// Expectation: Command-line symlink operands are resolved; nested symlinks remain links in the archive.
#[cfg(unix)]
#[test]
fn symlink_follow_command_line_partial() {
    setup();
    init_resource("symlink_follow_partial/source");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
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

/// Precondition: The source tree contains symlinks to both a file and a directory.
/// Action: Run `pna create` to create an archive.
/// Expectation: Each symlink entry carries fLTP metadata matching its target type.
#[test]
fn create_sets_fltp_on_symlinks() {
    setup();
    init_resource("create_sets_fltp_on_symlinks/source");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "create_sets_fltp_on_symlinks/test.pna",
        "--overwrite",
        "--keep-dir",
        "create_sets_fltp_on_symlinks/source",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut expected: HashMap<&str, Option<pna::LinkTargetType>> = HashMap::from([
        ("create_sets_fltp_on_symlinks/source", None),
        ("create_sets_fltp_on_symlinks/source/text.txt", None),
        ("create_sets_fltp_on_symlinks/source/dir", None),
        (
            "create_sets_fltp_on_symlinks/source/dir/in_dir_text.txt",
            None,
        ),
        (
            "create_sets_fltp_on_symlinks/source/link.txt",
            Some(pna::LinkTargetType::File),
        ),
        (
            "create_sets_fltp_on_symlinks/source/dir/in_dir_link.txt",
            Some(pna::LinkTargetType::File),
        ),
        (
            "create_sets_fltp_on_symlinks/source/link_dir",
            Some(pna::LinkTargetType::Directory),
        ),
    ]);
    archive::for_each_entry("create_sets_fltp_on_symlinks/test.pna", |entry| {
        let path = entry.header().path().to_string();
        let expected_ltp = expected
            .remove(path.as_str())
            .unwrap_or_else(|| panic!("unexpected entry found: {path}"));
        assert_eq!(entry.metadata().link_target_type(), expected_ltp, "{path}",);
    })
    .unwrap();
    assert!(
        expected.is_empty(),
        "missing expected entries: {expected:?}",
    );
}

/// Precondition: The source tree contains broken symlinks (targets do not exist).
/// Action: Run `pna create` to create an archive.
/// Expectation: On Unix the stat fallback hits `NotFound`, which degrades to
/// `Unknown` since the target is confirmed absent. On Windows the link-side
/// metadata classifies the symlink via its reparse-point flavor (File for
/// symlink_file, Directory for symlink_dir).
#[test]
fn create_broken_symlink_has_unknown_fltp() {
    setup();
    init_broken_resource("create_broken_symlink_has_unknown_fltp/source");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        "create_broken_symlink_has_unknown_fltp/test.pna",
        "--overwrite",
        "--keep-dir",
        "create_broken_symlink_has_unknown_fltp/source",
    ])
    .unwrap()
    .execute()
    .unwrap();

    #[cfg(windows)]
    let broken_ltp: Option<pna::LinkTargetType> = Some(pna::LinkTargetType::File);
    #[cfg(not(windows))]
    let broken_ltp: Option<pna::LinkTargetType> = Some(pna::LinkTargetType::Unknown);

    let mut expected: HashMap<&str, Option<pna::LinkTargetType>> = HashMap::from([
        ("create_broken_symlink_has_unknown_fltp/source", None),
        (
            "create_broken_symlink_has_unknown_fltp/source/broken.txt",
            broken_ltp,
        ),
        (
            "create_broken_symlink_has_unknown_fltp/source/broken_link",
            broken_ltp,
        ),
    ]);
    archive::for_each_entry("create_broken_symlink_has_unknown_fltp/test.pna", |entry| {
        let path = entry.header().path().to_string();
        let expected_ltp = expected
            .remove(path.as_str())
            .unwrap_or_else(|| panic!("unexpected entry found: {path}"));
        assert_eq!(entry.metadata().link_target_type(), expected_ltp, "{path}",);
    })
    .unwrap();
    assert!(
        expected.is_empty(),
        "missing expected entries: {expected:?}",
    );
}
