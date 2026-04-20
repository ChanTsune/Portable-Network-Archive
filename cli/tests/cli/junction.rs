//! Integration tests for Windows junction support.

use crate::utils::setup;
use clap::Parser;
use pna::{Archive, EntryBuilder, EntryName, EntryReference, LinkTargetType};
use portable_network_archive::cli;
use std::fs;

#[cfg(windows)]
use pna::{DataKind, ReadEntry, ReadOptions, prelude::*};

#[cfg(windows)]
fn mklink_junction(link: &std::path::Path, target: &std::path::Path) {
    let status = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .status()
        .expect("mklink");
    assert!(status.success(), "mklink /J failed");
}

/// Precondition: a directory tree containing a junction.
/// Action: `pna create` over the tree.
/// Expectation: the junction is encoded as HardLink + fLTP=Directory with the
/// absolute target path stored verbatim as entry data.
#[test]
#[cfg(windows)]
fn create_records_junction_as_hardlink_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("target_dir");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("inside.txt"), b"hi").unwrap();
    let junction = tmp.path().join("link_dir");
    mklink_junction(&junction, &target);

    let archive_path = tmp.path().join("out.pna");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_pna"))
        .current_dir(tmp.path())
        .args(["create", "-f"])
        .arg(&archive_path)
        .args(["link_dir", "target_dir"])
        .status()
        .unwrap();
    assert!(status.success());

    let bytes = std::fs::read(&archive_path).unwrap();
    let mut archive = Archive::read_header(&bytes[..]).unwrap();
    let mut saw_junction_entry = false;
    for entry in archive.entries_slice() {
        let entry = entry.unwrap();
        let ReadEntry::Normal(entry) = entry else {
            continue;
        };
        if entry.header().path().as_str() == "link_dir" {
            assert_eq!(entry.header().data_kind(), DataKind::HardLink);
            assert_eq!(
                entry.metadata().link_target_type(),
                Some(LinkTargetType::Directory)
            );
            let mut reader = entry.reader(ReadOptions::builder().build()).unwrap();
            let mut s = String::new();
            std::io::Read::read_to_string(&mut reader, &mut s).unwrap();
            let expected = target.to_string_lossy();
            assert_eq!(s, expected, "expected exact absolute target; got {s:?}");
            saw_junction_entry = true;
        }
    }
    assert!(saw_junction_entry, "no HardLink entry found for junction");
}

/// Build an in-memory archive containing one HardLink+fLTP=Directory entry
/// whose target is the supplied path string (interpreted verbatim).
fn build_junction_fixture(target: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut archive = Archive::write_header(&mut out).unwrap();
    let name = EntryName::from_utf8_preserve_root("link_dir");
    let reference = EntryReference::from_utf8_preserve_root(target);
    let mut builder = EntryBuilder::new_hard_link(name, reference).unwrap();
    builder.link_target_type(LinkTargetType::Directory);
    let entry = builder.build().unwrap();
    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();
    out
}

/// Precondition: archive with a HardLink+fLTP=Directory entry pointing at a
/// well-known absolute path.
/// Action: extract without `--allow-unsafe-links`.
/// Expectation: the entry is skipped and no link is created in the output
/// directory.
#[test]
fn extract_junction_without_allow_unsafe_links_skips() {
    setup();
    let base = "extract_junction_without_allow_unsafe_links_skips";
    let archive_path = format!("{base}/{base}.pna");
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&out_dir).unwrap();
    fs::write(&archive_path, build_junction_fixture("/any/absolute/path")).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(!std::path::Path::new(&out_dir).join("link_dir").exists());
}

/// Precondition: archive with a HardLink+fLTP=Directory entry pointing at an
/// existing absolute path that the test has materialized on disk.
/// Action: extract with `--allow-unsafe-links`.
/// Expectation: on Windows a real junction/reparse-point is created; on
/// non-Windows a symbolic link is created whose target string equals the
/// stored absolute path verbatim.
#[test]
fn extract_junction_with_allow_unsafe_links_creates_link() {
    setup();
    let base = "extract_junction_with_allow_unsafe_links_creates_link";
    let _ = fs::remove_dir_all(base);
    let target_rel = format!("{base}/actual_target");
    let out_dir = format!("{base}/out");
    fs::create_dir_all(&target_rel).unwrap();
    fs::create_dir_all(&out_dir).unwrap();
    // The fixture stores the junction target as an absolute path string.
    // `canonicalize` resolves any ancestor symlinks so that on Windows the
    // kernel accepts the path verbatim at `FSCTL_SET_REPARSE_POINT` time.
    let target_abs = fs::canonicalize(&target_rel).unwrap();
    let target_str = target_abs.to_string_lossy().into_owned();

    let archive_path = format!("{base}/{base}.pna");
    fs::write(&archive_path, build_junction_fixture(&target_str)).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &archive_path,
        "--out-dir",
        &out_dir,
        "--allow-unsafe-links",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let link = std::path::Path::new(&out_dir).join("link_dir");
    let meta = fs::symlink_metadata(&link).unwrap();

    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTypeExt;
        let ft = meta.file_type();
        assert!(
            ft.is_symlink() || ft.is_symlink_dir() || ft.is_symlink_file(),
            "expected a reparse-point flavored link at {}; got {:?}",
            link.display(),
            ft
        );
    }
    #[cfg(not(windows))]
    {
        assert!(meta.file_type().is_symlink());
        assert_eq!(
            fs::read_link(&link).unwrap(),
            std::path::PathBuf::from(&target_str)
        );
    }
}
