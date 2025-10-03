use crate::utils::{archive, setup};
use clap::Parser;
use portable_network_archive::{cli, command::Command};
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

#[cfg(unix)]
fn assert_symlink_follow(base: &str, follow_flag: &str) {
    let source_dir = format!("{base}/source");
    init_resource(&source_dir);

    let archive_path = format!("{base}/{base}.pna");
    cli::Cli::try_parse_from(vec![
        "pna".into(),
        "--quiet".into(),
        "c".into(),
        archive_path.clone(),
        "--overwrite".into(),
        "--keep-dir".into(),
        follow_flag.into(),
        source_dir.clone(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let prefix = format!("{base}/");
    archive::for_each_entry(&archive_path, |entry| {
        let path = entry.header().path().as_str();
        assert!(path.starts_with(&prefix), "unexpected entry path: {path}");
        let rel = &path[prefix.len()..];
        match rel {
            "source" => assert_eq!(entry.header().data_kind(), pna::DataKind::Directory),
            "source/text.txt" => assert_eq!(entry.header().data_kind(), pna::DataKind::File),
            "source/dir" => assert_eq!(entry.header().data_kind(), pna::DataKind::Directory),
            "source/dir/in_dir_text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "source/dir/in_dir_link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "source/link_dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory)
            }
            "source/link_dir/in_dir_link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "source/link_dir/in_dir_text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File)
            }
            "source/link.txt" => assert_eq!(entry.header().data_kind(), pna::DataKind::File),
            other => panic!("unexpected entry found: {other}"),
        }
    })
    .unwrap();

    let dist_dir = format!("{base}/dist");
    cli::Cli::try_parse_from(vec![
        "pna".into(),
        "--quiet".into(),
        "x".into(),
        archive_path,
        "--overwrite".into(),
        "--out-dir".into(),
        dist_dir.clone(),
        "--strip-components".into(),
        "2".into(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link_txt = PathBuf::from(format!("{dist_dir}/link.txt"));
    let link_dir = PathBuf::from(format!("{dist_dir}/link_dir"));
    let nested_link = PathBuf::from(format!("{dist_dir}/dir/in_dir_link.txt"));

    assert!(!link_txt.is_symlink());
    assert!(!link_dir.is_symlink());
    assert!(!nested_link.is_symlink());

    assert_eq!(
        fs::read_to_string(nested_link).unwrap(),
        fs::read_to_string(format!("{dist_dir}/dir/in_dir_text.txt")).unwrap()
    );
    assert_eq!(
        fs::read_to_string(format!("{dist_dir}/dir/in_dir_text.txt")).unwrap(),
        fs::read_to_string(format!("{dist_dir}/link_dir/in_dir_text.txt")).unwrap(),
    );
}

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
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink)
            }
            "symlink_no_follow/source/link_dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink)
            }
            "symlink_no_follow/source/link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink)
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
}

// FIXME: On GitHub Actions Windows runner disabled due to insufficient privileges for execution
#[cfg(unix)]
#[test]
fn symlink_follow() {
    setup();
    assert_symlink_follow("symlink_follow", "--follow-links");
}

#[cfg(unix)]
#[test]
fn symlink_follow_short_aliases() {
    setup();
    assert_symlink_follow("symlink_follow_short_L", "-L");
    assert_symlink_follow("symlink_follow_short_h", "-h");
}

#[cfg(unix)]
#[test]
fn follow_command_links_short_alias_without_unstable() {
    setup();
    init_resource("follow_command_links_short/source");

    let archive_path = "follow_command_links_short/archive.pna";
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        archive_path,
        "--overwrite",
        "-H",
        "follow_command_links_short/source/link_dir",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(Path::new(archive_path).exists());
}

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
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink)
            }
            "broken_symlink_no_follow/source/broken_dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink)
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
}

// FIXME: On GitHub Actions Windows runner disabled due to insufficient privileges for execution
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
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink)
            }
            "broken_symlink_follow/source/broken_dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink)
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
}

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
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink)
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
}

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
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink)
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
}

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
            }
            "symlink_follow_partial/source/dir/in_dir_text.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::File);
            }
            "symlink_follow_partial/source/link_dir" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::Directory);
            }
            "symlink_follow_partial/source/link_dir/in_dir_link.txt" => {
                assert_eq!(entry.header().data_kind(), pna::DataKind::SymbolicLink);
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
