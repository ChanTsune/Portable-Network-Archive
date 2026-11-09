use crate::utils::setup;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::{
    Archive, EntryName, EntryReference, FileEntryBuilder, HardLinkEntryBuilder,
    SymlinkEntryBuilder, WriteOptions,
};
use std::{fs, io::Write, path::Path, path::PathBuf};

fn build_archive_with_file_and_parent_dir_links(
    archive_path: &Path,
    include_parent_dir_hardlink: bool,
) {
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(archive_path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();

    writer
        .add_entry({
            let mut builder = FileEntryBuilder::new_with_options(
                EntryName::from_utf8_preserve_root("./file.txt"),
                WriteOptions::builder().build(),
            )
            .unwrap();
            builder.write_all(b"content").unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer
        .add_entry({
            SymlinkEntryBuilder::new(
                EntryName::from_utf8_preserve_root("./a/link"),
                EntryReference::from_utf8_preserve_root("../file.txt"),
            )
            .unwrap()
            .build()
            .unwrap()
        })
        .unwrap();

    if include_parent_dir_hardlink {
        writer
            .add_entry({
                HardLinkEntryBuilder::new(
                    EntryName::from_utf8_preserve_root("./b/hard"),
                    EntryReference::from_utf8_preserve_root("../file.txt"),
                )
                .unwrap()
                .build()
                .unwrap()
            })
            .unwrap();
    }

    writer.finalize().unwrap();
}

/// Precondition: Archive contains a file and a symlink whose target uses parent directory traversal (..)
/// Action: Extract with bsdtar -x and --allow-unsafe-links
/// Expectation: Extraction succeeds and the symlink is created with the target preserved verbatim
#[test]
fn bsdtar_extract_allow_unsafe_links_flag_allows_parent_dir_symlink_target() {
    setup();

    let root = PathBuf::from("bsdtar_extract_allow_unsafe_links_flag_allows_parent_dir_target");
    let archive_path = root.join("archive.pna");
    let out_dir = root.join("out");

    build_archive_with_file_and_parent_dir_links(&archive_path, false);

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--extract",
            "--unstable",
            "--overwrite",
            "--allow-unsafe-links",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .success();

    let link_path = out_dir.join("a/link");
    let meta = fs::symlink_metadata(&link_path).unwrap();
    assert!(meta.file_type().is_symlink(), "symlink should be created");
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        Path::new("../file.txt"),
        "symlink target should be preserved verbatim"
    );
    assert_eq!(
        fs::read_to_string(&link_path).unwrap(),
        "content",
        "symlink should resolve to the extracted file"
    );
}

/// Precondition: Archive contains a file, a symlink whose target uses parent directory traversal (..), and a hardlink whose target uses parent directory traversal (..)
/// Action: Extract with bsdtar -x and --no-allow-unsafe-links
/// Expectation: Extraction fails; the symlink and the hardlink are not created and the regular file is extracted
#[test]
fn bsdtar_extract_no_allow_unsafe_links_flag_blocks_parent_dir_link_targets() {
    setup();

    let root = PathBuf::from("bsdtar_extract_no_allow_unsafe_links_flag_blocks_parent_dir_targets");
    let archive_path = root.join("archive.pna");
    let out_dir = root.join("out");

    build_archive_with_file_and_parent_dir_links(&archive_path, true);

    cargo_bin_cmd!("pna")
        .args([
            "--quiet",
            "compat",
            "bsdtar",
            "--extract",
            "--unstable",
            "--overwrite",
            "--no-allow-unsafe-links",
            "--file",
            archive_path.to_str().unwrap(),
            "--out-dir",
            out_dir.to_str().unwrap(),
        ])
        .assert()
        .failure();

    assert!(
        out_dir.join("file.txt").exists(),
        "regular file should be extracted"
    );
    assert!(
        !out_dir.join("a/link").exists(),
        "symlink with .. target should be blocked"
    );
    assert!(
        !out_dir.join("b/hard").exists(),
        "hardlink with .. target should be blocked"
    );
}
