use crate::utils::setup;
use clap::Parser;
use pna::{
    Archive, DirEntryBuilder, Duration, FileEntryBuilder, Metadata, SymlinkEntryBuilder,
    WriteOptions, fs as pna_fs,
};
use portable_network_archive::cli;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::SystemTime,
};

fn init_file_archive<P: AsRef<Path>>(path: P, entry_name: &str, modified: Option<Duration>) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let file = fs::File::create(path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    let mut builder =
        FileEntryBuilder::new_with_options(entry_name.into(), WriteOptions::builder().build())
            .unwrap();
    if let Some(mtime) = modified {
        builder.metadata(Metadata::new().with_modified(Some(mtime)));
    }
    builder.write_all(b"from archive").unwrap();
    let entry = builder.build().unwrap();
    archive.add_entry(entry).unwrap();
    archive.finalize().unwrap();
}

fn init_dir_archive<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let file = fs::File::create(path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    archive
        .add_entry(DirEntryBuilder::new("a/b".into()).build().unwrap())
        .unwrap();
    archive.finalize().unwrap();
}

fn init_symlink_archive<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let file = fs::File::create(path).unwrap();
    let mut archive = Archive::write_header(file).unwrap();
    archive
        .add_entry(
            SymlinkEntryBuilder::new("a/link".into(), "target.txt".into())
                .unwrap()
                .build()
                .unwrap(),
        )
        .unwrap();
    archive.finalize().unwrap();
}

fn init_out_dir(path: &str) -> PathBuf {
    let out_dir = PathBuf::from(path);
    if out_dir.symlink_metadata().is_ok() {
        pna_fs::remove_path_all(&out_dir).unwrap();
    }
    fs::create_dir_all(&out_dir).unwrap();
    out_dir
}

/// Precondition: An archive entry is nested below a path where a regular file already exists.
/// Action: Extract via `pna extract` with the default overwrite strategy.
/// Expectation: Extraction fails and the existing regular file is preserved.
#[test]
fn parent_file_blocks_extract_by_default() {
    setup();
    init_file_archive("parent_file/default/archive.pna", "a/b.txt", None);

    let out_dir = init_out_dir("parent_file/default/out");
    let blocking = out_dir.join("a");
    fs::write(&blocking, b"existing").unwrap();

    let error = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        "parent_file/default/out",
        "-f",
        "parent_file/default/archive.pna",
    ])
    .unwrap()
    .execute()
    .unwrap_err();

    let message = format!("{error:#}");
    assert!(message.contains("parent_file/default/out/a"), "{message}");
    assert!(message.contains("already exists"), "{message}");
    assert_eq!(fs::read(&blocking).unwrap(), b"existing");
}

/// Precondition: An archive entry is nested two levels below a path where a regular file already exists.
/// Action: Extract via `pna extract` with the default overwrite strategy.
/// Expectation: Extraction fails and the existing regular file is preserved.
#[test]
fn grandparent_file_blocks_extract_by_default() {
    setup();
    init_file_archive("parent_file/grandparent/archive.pna", "a/b/c.txt", None);

    let out_dir = init_out_dir("parent_file/grandparent/out");
    let blocking = out_dir.join("a");
    fs::write(&blocking, b"existing").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        "parent_file/grandparent/out",
        "-f",
        "parent_file/grandparent/archive.pna",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert_eq!(fs::read(&blocking).unwrap(), b"existing");
}

/// Precondition: An archive entry is nested below a directory that contains a regular file at the intermediate path.
/// Action: Extract via `pna extract` with the default overwrite strategy.
/// Expectation: Extraction fails and the intermediate regular file is preserved.
#[test]
fn intermediate_file_blocks_extract_by_default() {
    setup();
    init_file_archive("parent_file/intermediate/archive.pna", "a/b/c.txt", None);

    let out_dir = init_out_dir("parent_file/intermediate/out");
    fs::create_dir_all(out_dir.join("a")).unwrap();
    let blocking = out_dir.join("a/b");
    fs::write(&blocking, b"existing").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        "parent_file/intermediate/out",
        "-f",
        "parent_file/intermediate/archive.pna",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert_eq!(fs::read(&blocking).unwrap(), b"existing");
}

/// Precondition: An archive entry is nested below a path where a regular file already exists.
/// Action: Extract via `pna extract` with `--overwrite`.
/// Expectation: The regular file is replaced by a directory and the entry is extracted.
#[test]
fn parent_file_replaced_with_overwrite() {
    setup();
    init_file_archive("parent_file/overwrite/archive.pna", "a/b.txt", None);

    let out_dir = init_out_dir("parent_file/overwrite/out");
    let blocking = out_dir.join("a");
    fs::write(&blocking, b"existing").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        "parent_file/overwrite/out",
        "--overwrite",
        "-f",
        "parent_file/overwrite/archive.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(blocking.is_dir());
    assert_eq!(fs::read(out_dir.join("a/b.txt")).unwrap(), b"from archive");
}

/// Precondition: An archive entry is nested below a path where a regular file already exists.
/// Action: Extract via `pna extract` with `--keep-old-files`.
/// Expectation: Extraction succeeds, the entry is skipped, and the existing regular file is preserved.
#[test]
fn parent_file_preserved_with_keep_old_files() {
    setup();
    init_file_archive("parent_file/keep_older/archive.pna", "a/b.txt", None);

    let out_dir = init_out_dir("parent_file/keep_older/out");
    let blocking = out_dir.join("a");
    fs::write(&blocking, b"existing").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        "parent_file/keep_older/out",
        "--keep-old-files",
        "-f",
        "parent_file/keep_older/archive.pna",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(fs::read(&blocking).unwrap(), b"existing");
}

/// Precondition: An archive entry is newer than a regular file that blocks its parent path.
/// Action: Extract via `pna extract` with `--keep-newer-files`.
/// Expectation: Extraction succeeds, the entry is skipped, and the blocking regular file is preserved because it is not the entry itself.
#[test]
fn parent_file_preserved_with_keep_newer_files() {
    setup();
    init_file_archive(
        "parent_file/keep_newer/archive.pna",
        "a/b.txt",
        Some(Duration::seconds(1)),
    );

    let out_dir = init_out_dir("parent_file/keep_newer/out");
    let blocking = out_dir.join("a");
    fs::write(&blocking, b"existing").unwrap();

    let file = fs::File::options().write(true).open(&blocking).unwrap();
    file.set_modified(SystemTime::UNIX_EPOCH).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        "parent_file/keep_newer/out",
        "--keep-newer-files",
        "-f",
        "parent_file/keep_newer/archive.pna",
        "--unstable",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(fs::read(&blocking).unwrap(), b"existing");
}

/// Precondition: A directory entry is nested below a path where a regular file already exists.
/// Action: Extract via `pna extract` with the default overwrite strategy.
/// Expectation: Extraction fails and the existing regular file is preserved.
#[test]
fn parent_file_blocks_directory_entry_by_default() {
    setup();
    init_dir_archive("parent_file/dir_default/archive.pna");

    let out_dir = init_out_dir("parent_file/dir_default/out");
    let blocking = out_dir.join("a");
    fs::write(&blocking, b"existing").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        "parent_file/dir_default/out",
        "-f",
        "parent_file/dir_default/archive.pna",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert_eq!(fs::read(&blocking).unwrap(), b"existing");
}

/// Precondition: A directory entry is nested below a path where a regular file already exists.
/// Action: Extract via `pna extract` with `--overwrite`.
/// Expectation: The regular file is replaced by directories and the directory entry is extracted.
#[test]
fn parent_file_replaced_for_directory_entry_with_overwrite() {
    setup();
    init_dir_archive("parent_file/dir_overwrite/archive.pna");

    let out_dir = init_out_dir("parent_file/dir_overwrite/out");
    let blocking = out_dir.join("a");
    fs::write(&blocking, b"existing").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        "parent_file/dir_overwrite/out",
        "--overwrite",
        "-f",
        "parent_file/dir_overwrite/archive.pna",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert!(blocking.is_dir());
    assert!(out_dir.join("a/b").is_dir());
}

/// Precondition: A symlink entry is nested below a path where a regular file already exists.
/// Action: Extract via `pna extract` with the default overwrite strategy.
/// Expectation: Extraction fails and the existing regular file is preserved.
#[test]
fn parent_file_blocks_symlink_entry_by_default() {
    setup();
    init_symlink_archive("parent_file/symlink_default/archive.pna");

    let out_dir = init_out_dir("parent_file/symlink_default/out");
    let blocking = out_dir.join("a");
    fs::write(&blocking, b"existing").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--out-dir",
        "parent_file/symlink_default/out",
        "-f",
        "parent_file/symlink_default/archive.pna",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert_eq!(fs::read(&blocking).unwrap(), b"existing");
}
