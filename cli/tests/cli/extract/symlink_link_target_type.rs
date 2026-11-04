use crate::utils::{
    archive::{SymlinkEntryDef, create_archive_with_symlinks},
    setup,
};
use clap::Parser;
use portable_network_archive::cli;
use std::{fs, path::PathBuf};

/// Precondition: Archive contains a symlink entry whose fLTP says Directory
/// and whose target does not exist on disk at extraction time.
/// Action: Extract.
/// Expectation: The extracted link is a directory symlink (symlink_dir
/// flavor) because the extractor honored fLTP instead of probing the
/// target's actual file type.
#[test]
fn extract_symlink_with_fltp_directory_uses_symlink_dir() {
    setup();
    let base = PathBuf::from("extract_symlink_with_fltp_directory_uses_symlink_dir");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    let archive_path = base.join("input.pna");
    create_archive_with_symlinks(
        &archive_path,
        &[],
        &[SymlinkEntryDef {
            path: "link_to_dir",
            target: "missing_dir",
            permission: None,
            modified: None,
            accessed: None,
            created: None,
            link_target_type: Some(pna::LinkTargetType::Directory),
        }],
    )
    .unwrap();

    let out_dir = base.join("out");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--overwrite",
        "--out-dir",
        out_dir.to_str().unwrap(),
        "-f",
        archive_path.to_str().unwrap(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    use std::os::windows::fs::FileTypeExt;
    let link_path = out_dir.join("link_to_dir");
    let link_meta = fs::symlink_metadata(&link_path).unwrap();
    assert!(
        link_meta.file_type().is_symlink_dir(),
        "expected symlink_dir flavor; got {:?}",
        link_meta.file_type(),
    );
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("missing_dir")
    );
}

/// Precondition: Archive contains a symlink entry whose fLTP says File
/// and whose target does not exist on disk at extraction time.
/// Action: Extract.
/// Expectation: The extracted link is a file symlink (symlink_file flavor).
#[test]
fn extract_symlink_with_fltp_file_uses_symlink_file() {
    setup();
    let base = PathBuf::from("extract_symlink_with_fltp_file_uses_symlink_file");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    let archive_path = base.join("input.pna");
    create_archive_with_symlinks(
        &archive_path,
        &[],
        &[SymlinkEntryDef {
            path: "link_to_file",
            target: "missing_file",
            permission: None,
            modified: None,
            accessed: None,
            created: None,
            link_target_type: Some(pna::LinkTargetType::File),
        }],
    )
    .unwrap();

    let out_dir = base.join("out");
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--overwrite",
        "--out-dir",
        out_dir.to_str().unwrap(),
        "-f",
        archive_path.to_str().unwrap(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    use std::os::windows::fs::FileTypeExt;
    let link_path = out_dir.join("link_to_file");
    let link_meta = fs::symlink_metadata(&link_path).unwrap();
    assert!(
        link_meta.file_type().is_symlink_file(),
        "expected symlink_file flavor; got {:?}",
        link_meta.file_type(),
    );
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("missing_file")
    );
}

/// Precondition: Archive contains a symlink entry whose fLTP says File,
/// and the target already exists as a directory under the extraction root.
/// Action: Extract.
/// Expectation: The extractor honors fLTP=File and creates a file symlink
/// even though the filesystem target is a directory. Discriminating case:
/// the archive-declared fLTP contradicts the filesystem target's actual
/// type, so probing the filesystem would yield the wrong flavor.
#[test]
fn extract_symlink_with_fltp_file_overrides_existing_directory_target() {
    setup();
    let base = PathBuf::from("extract_symlink_with_fltp_file_overrides_existing_directory_target");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    let archive_path = base.join("input.pna");
    create_archive_with_symlinks(
        &archive_path,
        &[],
        &[SymlinkEntryDef {
            path: "link",
            target: "existing_dir",
            permission: None,
            modified: None,
            accessed: None,
            created: None,
            link_target_type: Some(pna::LinkTargetType::File),
        }],
    )
    .unwrap();

    let out_dir = base.join("out");
    fs::create_dir_all(out_dir.join("existing_dir")).unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--overwrite",
        "--out-dir",
        out_dir.to_str().unwrap(),
        "-f",
        archive_path.to_str().unwrap(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    use std::os::windows::fs::FileTypeExt;
    let link_path = out_dir.join("link");
    let link_meta = fs::symlink_metadata(&link_path).unwrap();
    assert!(
        link_meta.file_type().is_symlink_file(),
        "expected symlink_file flavor (fLTP=File overrides existing dir target); got {:?}",
        link_meta.file_type(),
    );
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("existing_dir")
    );
}

/// Precondition: Archive contains a symlink entry whose fLTP says Directory,
/// and the target already exists as a regular file under the extraction root.
/// Action: Extract.
/// Expectation: The extractor honors fLTP=Directory and creates a directory
/// symlink even though the filesystem target is a regular file. Discriminating
/// case: the archive-declared fLTP contradicts the filesystem target's actual
/// type, so probing the filesystem would yield the wrong flavor.
#[test]
fn extract_symlink_with_fltp_directory_overrides_existing_file_target() {
    setup();
    let base = PathBuf::from("extract_symlink_with_fltp_directory_overrides_existing_file_target");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    let archive_path = base.join("input.pna");
    create_archive_with_symlinks(
        &archive_path,
        &[],
        &[SymlinkEntryDef {
            path: "link",
            target: "existing_file",
            permission: None,
            modified: None,
            accessed: None,
            created: None,
            link_target_type: Some(pna::LinkTargetType::Directory),
        }],
    )
    .unwrap();

    let out_dir = base.join("out");
    fs::create_dir_all(&out_dir).unwrap();
    fs::write(out_dir.join("existing_file"), b"data").unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "x",
        "--overwrite",
        "--out-dir",
        out_dir.to_str().unwrap(),
        "-f",
        archive_path.to_str().unwrap(),
    ])
    .unwrap()
    .execute()
    .unwrap();

    use std::os::windows::fs::FileTypeExt;
    let link_path = out_dir.join("link");
    let link_meta = fs::symlink_metadata(&link_path).unwrap();
    assert!(
        link_meta.file_type().is_symlink_dir(),
        "expected symlink_dir flavor (fLTP=Directory overrides existing file target); got {:?}",
        link_meta.file_type(),
    );
    assert_eq!(
        fs::read_link(&link_path).unwrap(),
        PathBuf::from("existing_file")
    );
}
