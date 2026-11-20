use crate::utils::setup;
use clap::Parser;
use pna::{
    Archive, FileEntryBuilder, HardLinkEntryBuilder, SymlinkEntryBuilder, WriteOptions,
    fs as pna_fs,
};
use portable_network_archive::cli;
use std::{fs, io::Write, path::Path};

fn init_resource<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();

    writer
        .add_entry({
            let mut builder = FileEntryBuilder::new_with_options(
                "origin1.txt".into(),
                WriteOptions::builder().build(),
            )
            .unwrap();
            builder.write_all(b"original text\n").unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder =
                HardLinkEntryBuilder::new("linked1.txt".into(), "origin1.txt".into()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder =
                HardLinkEntryBuilder::new("dir/linked1.txt".into(), "origin1.txt".into()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer
        .add_entry({
            let mut builder = FileEntryBuilder::new_with_options(
                "dir/origin2.txt".into(),
                WriteOptions::builder().build(),
            )
            .unwrap();
            builder.write_all(b"original text text\n").unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder =
                HardLinkEntryBuilder::new("dir/linked2.txt".into(), "dir/origin2.txt".into())
                    .unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder =
                HardLinkEntryBuilder::new("linked2.txt".into(), "dir/origin2.txt".into()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();

    writer.finalize().unwrap();
}

#[test]
fn hardlink_extract() {
    setup();
    init_resource("hardlink_extract/hardlink.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "hardlink_extract/hardlink.pna",
        "--overwrite",
        "--out-dir",
        "hardlink_extract/dist",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        "original text\n",
        fs::read_to_string("hardlink_extract/dist/linked1.txt").unwrap()
    );

    assert_eq!(
        "original text\n",
        fs::read_to_string("hardlink_extract/dist/dir/linked1.txt",).unwrap()
    );
    #[cfg(not(target_family = "wasm"))]
    assert!(
        same_file::is_same_file(
            "hardlink_extract/dist/linked1.txt",
            "hardlink_extract/dist/dir/linked1.txt"
        )
        .unwrap()
    );

    assert_eq!(
        "original text text\n",
        fs::read_to_string("hardlink_extract/dist/dir/linked2.txt").unwrap()
    );
    assert_eq!(
        "original text text\n",
        fs::read_to_string("hardlink_extract/dist/linked2.txt").unwrap()
    );
    #[cfg(not(target_family = "wasm"))]
    assert!(
        same_file::is_same_file(
            "hardlink_extract/dist/dir/linked2.txt",
            "hardlink_extract/dist/linked2.txt",
        )
        .unwrap()
    );
}

#[test]
fn hardlink_extract_allow_unsafe_links() {
    setup();
    init_resource("hardlink_extract_allow_unsafe_links/hardlink.pna");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        "hardlink_extract_allow_unsafe_links/hardlink.pna",
        "--allow-unsafe-links",
        "--overwrite",
        "--out-dir",
        "hardlink_extract_allow_unsafe_links/dist",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        "original text\n",
        fs::read_to_string("hardlink_extract_allow_unsafe_links/dist/linked1.txt",).unwrap()
    );
    assert_eq!(
        "original text\n",
        fs::read_to_string("hardlink_extract_allow_unsafe_links/dist/dir/linked1.txt",).unwrap()
    );
    #[cfg(not(target_family = "wasm"))]
    assert!(
        same_file::is_same_file(
            "hardlink_extract_allow_unsafe_links/dist/linked1.txt",
            "hardlink_extract_allow_unsafe_links/dist/dir/linked1.txt"
        )
        .unwrap()
    );

    assert_eq!(
        "original text text\n",
        fs::read_to_string("hardlink_extract_allow_unsafe_links/dist/dir/linked2.txt",).unwrap()
    );
    assert_eq!(
        "original text text\n",
        fs::read_to_string("hardlink_extract_allow_unsafe_links/dist/linked2.txt",).unwrap()
    );
    #[cfg(not(target_family = "wasm"))]
    assert!(
        same_file::is_same_file(
            "hardlink_extract_allow_unsafe_links/dist/dir/linked2.txt",
            "hardlink_extract_allow_unsafe_links/dist/linked2.txt",
        )
        .unwrap()
    );
}

fn init_hardlink_through_symlink_archive<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    let file = fs::File::create(path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();
    writer
        .add_entry({
            let builder =
                HardLinkEntryBuilder::new("esc.txt".into(), "linkdir/secret.txt".into()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer.finalize().unwrap();
}

fn init_outside_secret_tree(root: &Path) {
    if fs::exists(root).unwrap() {
        fs::remove_dir_all(root).unwrap();
    }
    fs::create_dir_all(root.join("outside")).unwrap();
    fs::write(root.join("outside/secret.txt"), b"outside secret\n").unwrap();
    fs::create_dir_all(root.join("dist")).unwrap();
    pna_fs::symlink(Path::new("../outside"), root.join("dist/linkdir")).unwrap();
}

/// Precondition: The extraction directory contains a symlink to a directory outside it, and the archive contains a hardlink whose target path passes through that symlink
/// Action: Extract without --allow-unsafe-links
/// Expectation: Extraction fails, the hardlink is not created, and the outside file is untouched
#[test]
fn hardlink_through_symlink_parent_is_skipped() {
    setup();
    let root = Path::new("hardlink_through_symlink_parent_is_skipped");
    init_outside_secret_tree(root);
    init_hardlink_through_symlink_archive(root.join("archive.pna"));

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &root.join("archive.pna").to_string_lossy(),
        "--overwrite",
        "--out-dir",
        &root.join("dist").to_string_lossy(),
    ])
    .unwrap()
    .execute();

    assert!(
        result.is_err(),
        "extraction should fail when a hardlink is blocked"
    );
    assert!(
        !fs::exists(root.join("dist/esc.txt")).unwrap(),
        "hardlink through a symlink parent should not be created"
    );
    assert_eq!(
        fs::read(root.join("outside/secret.txt")).unwrap(),
        b"outside secret\n",
        "file outside the extraction directory should be untouched"
    );
}

/// Precondition: The extraction directory contains a symlink to a directory outside it, and the archive contains a hardlink whose target path passes through that symlink
/// Action: Extract with --allow-unsafe-links
/// Expectation: Extraction succeeds and the hardlink is created to the outside file
#[test]
fn hardlink_through_symlink_parent_with_allow_unsafe_links() {
    setup();
    let root = Path::new("hardlink_through_symlink_parent_with_allow_unsafe_links");
    init_outside_secret_tree(root);
    init_hardlink_through_symlink_archive(root.join("archive.pna"));

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &root.join("archive.pna").to_string_lossy(),
        "--allow-unsafe-links",
        "--overwrite",
        "--out-dir",
        &root.join("dist").to_string_lossy(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        fs::read(root.join("dist/esc.txt")).unwrap(),
        b"outside secret\n",
        "hardlink should resolve to the outside file"
    );
    #[cfg(not(target_family = "wasm"))]
    assert!(
        same_file::is_same_file(root.join("dist/esc.txt"), root.join("outside/secret.txt"))
            .unwrap(),
        "hardlink should share the outside file's inode"
    );
}

/// Precondition: The archive contains a file, a symlink to a directory inside the extraction directory, and a hardlink whose target path passes through that symlink
/// Action: Extract without --allow-unsafe-links
/// Expectation: Extraction fails and the hardlink is not created, while the file and the symlink are extracted
#[test]
fn hardlink_through_archive_symlink_parent_is_skipped() {
    setup();
    let root = Path::new("hardlink_through_archive_symlink_parent_is_skipped");
    let archive_path = root.join("archive.pna");
    if fs::exists(root).unwrap() {
        fs::remove_dir_all(root).unwrap();
    }
    fs::create_dir_all(root).unwrap();
    let file = fs::File::create(&archive_path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();
    writer
        .add_entry({
            let mut builder = FileEntryBuilder::new_with_options(
                "subdir/secret.txt".into(),
                WriteOptions::builder().build(),
            )
            .unwrap();
            builder.write_all(b"inside secret\n").unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder = SymlinkEntryBuilder::new("linkdir".into(), "subdir".into()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder =
                HardLinkEntryBuilder::new("esc.txt".into(), "linkdir/secret.txt".into()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer.finalize().unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &archive_path.to_string_lossy(),
        "--overwrite",
        "--out-dir",
        &root.join("dist").to_string_lossy(),
    ])
    .unwrap()
    .execute();

    assert!(
        result.is_err(),
        "extraction should fail when a hardlink is blocked"
    );
    assert_eq!(
        fs::read(root.join("dist/subdir/secret.txt")).unwrap(),
        b"inside secret\n",
        "regular file should be extracted"
    );
    assert!(
        fs::symlink_metadata(root.join("dist/linkdir"))
            .unwrap()
            .file_type()
            .is_symlink(),
        "symlink should be extracted"
    );
    assert!(
        !fs::exists(root.join("dist/esc.txt")).unwrap(),
        "hardlink through a symlink parent should not be created"
    );
}

/// Precondition: The archive contains a file, a symlink to that file, and a hardlink whose target is the symlink itself
/// Action: Extract without --allow-unsafe-links
/// Expectation: Extraction succeeds and the hardlink is created as another name for the symlink
#[test]
fn hardlink_to_symlink_target_is_created() {
    setup();
    let root = Path::new("hardlink_to_symlink_target_is_created");
    let archive_path = root.join("archive.pna");
    if fs::exists(root).unwrap() {
        fs::remove_dir_all(root).unwrap();
    }
    fs::create_dir_all(root).unwrap();
    let file = fs::File::create(&archive_path).unwrap();
    let mut writer = Archive::write_header(file).unwrap();
    writer
        .add_entry({
            let mut builder = FileEntryBuilder::new_with_options(
                "real.txt".into(),
                WriteOptions::builder().build(),
            )
            .unwrap();
            builder.write_all(b"real content\n").unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder = SymlinkEntryBuilder::new("link.txt".into(), "real.txt".into()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer
        .add_entry({
            let builder = HardLinkEntryBuilder::new("alias.txt".into(), "link.txt".into()).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    writer.finalize().unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "-f",
        &archive_path.to_string_lossy(),
        "--overwrite",
        "--out-dir",
        &root.join("dist").to_string_lossy(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    let alias = root.join("dist/alias.txt");
    assert!(
        fs::symlink_metadata(&alias)
            .unwrap()
            .file_type()
            .is_symlink(),
        "hardlink to a symlink should be created as a symlink"
    );
    assert_eq!(
        fs::read_link(&alias).unwrap(),
        Path::new("real.txt"),
        "created link should preserve the symlink target"
    );
    assert_eq!(
        fs::read_to_string(&alias).unwrap(),
        "real content\n",
        "created link should resolve to the file content"
    );
}
