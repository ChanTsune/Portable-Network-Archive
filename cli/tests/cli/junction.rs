//! Integration tests for Windows junction support.

use std::process::Command;

use pna::{Archive, EntryBuilder, EntryName, EntryReference, LinkTargetType};

#[cfg(windows)]
use pna::{DataKind, ReadEntry, ReadOptions, prelude::*};

#[cfg(windows)]
fn mklink_junction(link: &std::path::Path, target: &std::path::Path) {
    let status = Command::new("cmd")
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
    let status = Command::new(env!("CARGO_BIN_EXE_pna"))
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
/// Expectation: the entry is skipped with a warning and no link is created.
#[test]
fn extract_junction_without_allow_unsafe_links_skips() {
    let tmp = tempfile::tempdir().unwrap();
    // Canonicalize to resolve any ancestor symlinks (e.g. `/var` -> `/private/var`
    // on macOS) so extract's secure-symlink ancestor check does not trip on the
    // temp dir's path.
    let tmp_root = std::fs::canonicalize(tmp.path()).unwrap();
    let archive_path = tmp_root.join("fixture.pna");
    std::fs::write(&archive_path, build_junction_fixture("/any/absolute/path")).unwrap();

    let out_dir = tmp_root.join("out");
    std::fs::create_dir(&out_dir).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_pna"))
        .args(["extract", "-f"])
        .arg(&archive_path)
        .arg("--out-dir")
        .arg(&out_dir)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!out_dir.join("link_dir").exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unsafe") || stderr.contains("allow-unsafe-links"),
        "expected skip warning, got: {stderr}"
    );
}
