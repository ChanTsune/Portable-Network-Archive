use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use pna::prelude::*;
use portable_network_archive::cli;

const ENTRY_PATH: &str = "test.txt";
const ENTRY_CONTENT: &[u8] = b"test content";

/// Precondition: A solid archive contains a file with permission 0o777 (rwxrwxrwx).
/// Action: Run `pna experimental chmod` with `--keep-solid` and `-x` to remove execute.
/// Expectation: The archive entry's permission becomes 0o666 (rw-rw-rw-) and archive remains solid.
#[test]
fn chmod_keep_solid() {
    setup();

    archive::create_solid_archive_with_permissions(
        "chmod_keep_solid.pna",
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o777,
        }],
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "--keep-solid",
        "-f",
        "chmod_keep_solid.pna",
        "--",
        "-x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut found = false;
    archive::for_each_entry("chmod_keep_solid.pna", |entry| {
        if entry.header().path() == ENTRY_PATH {
            found = true;
            let mode = entry
                .metadata()
                .permission_mode()
                .expect("entry should have permission mode metadata")
                .get();
            assert_eq!(mode & 0o777, 0o666, "-x on 0o777 should yield 0o666");
        }
    })
    .unwrap();
    assert!(found, "target entry not found in archive");
}

fn solid_headers(path: &str) -> Vec<(pna::Compression, pna::Encryption, pna::CipherMode)> {
    let mut archive = pna::Archive::open(path).unwrap();
    archive
        .entries()
        .map(|entry| match entry.unwrap() {
            pna::ReadEntry::Solid(s) => {
                let header = s.header();
                (
                    header.compression(),
                    header.encryption(),
                    header.cipher_mode(),
                )
            }
            pna::ReadEntry::Normal(_) => panic!("expected a solid entry"),
        })
        .collect()
}

/// Precondition: An encrypted solid archive contains a file entry.
/// Action: Run `pna experimental chmod` with `--keep-solid` and the correct password.
/// Expectation: The permission change is applied, the archive remains solid with its
///              compression, encryption, and cipher mode unchanged, and it still decrypts
///              with the same password.
#[test]
fn chmod_keep_solid_on_encrypted_solid_archive() {
    setup();

    archive::create_encrypted_solid_archive_with_permissions(
        "chmod_keep_solid_encrypted.pna",
        &[FileEntryDef {
            path: ENTRY_PATH,
            content: ENTRY_CONTENT,
            permission: 0o777,
        }],
        "password",
    )
    .unwrap();
    let headers_before = solid_headers("chmod_keep_solid_encrypted.pna");
    assert_eq!(headers_before.len(), 1);

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "--keep-solid",
        "-f",
        "chmod_keep_solid_encrypted.pna",
        "--password",
        "password",
        "--",
        "-x",
        ENTRY_PATH,
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        archive::entry_mode_with_password(
            "chmod_keep_solid_encrypted.pna",
            ENTRY_PATH,
            Some("password"),
        ),
        0o666,
    );
    assert_eq!(
        solid_headers("chmod_keep_solid_encrypted.pna"),
        headers_before
    );
}

fn stored_file_entry(path: &str, permission: u16) -> pna::NormalEntry {
    use std::io::Write;

    let mut builder =
        pna::FileEntryBuilder::new_with_options(path.into(), pna::WriteOptions::store()).unwrap();
    builder.metadata(
        pna::Metadata::new().with_permission_mode(Some(pna::PermissionMode::from(permission))),
    );
    builder.write_all(ENTRY_CONTENT).unwrap();
    builder.build().unwrap()
}

/// Precondition: An archive interleaves normal entries around one solid block.
/// Action: Run `pna experimental chmod` with `--keep-solid` over every entry.
/// Expectation: The entry order and the normal/solid layout survive, and the permission
///              change is applied to every entry.
#[test]
fn chmod_keep_solid_preserves_mixed_normal_and_solid_layout() {
    setup();

    let path = "chmod_keep_solid_mixed.pna";
    let file = std::fs::File::create(path).unwrap();
    let mut out = pna::Archive::write_header(file).unwrap();
    out.add_entry(stored_file_entry("outer/before.txt", 0o777))
        .unwrap();
    let mut solid = pna::SolidEntryBuilder::new(pna::WriteOptions::store()).unwrap();
    solid
        .add_entry(stored_file_entry("inner/a.txt", 0o777))
        .unwrap();
    solid
        .add_entry(stored_file_entry("inner/b.txt", 0o777))
        .unwrap();
    out.add_entry(solid.build().unwrap()).unwrap();
    out.add_entry(stored_file_entry("outer/after.txt", 0o777))
        .unwrap();
    out.finalize().unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "experimental",
        "chmod",
        "--keep-solid",
        "-f",
        path,
        "--",
        "-x",
        "outer/before.txt",
        "inner/a.txt",
        "inner/b.txt",
        "outer/after.txt",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut archive = pna::Archive::open(path).unwrap();
    let layout = archive
        .entries()
        .map(|entry| matches!(entry.unwrap(), pna::ReadEntry::Solid(_)))
        .collect::<Vec<_>>();
    assert_eq!(layout, [false, true, false]);

    let mut entries = Vec::new();
    archive::for_each_entry(path, |entry| {
        entries.push((
            entry.header().path().to_string(),
            entry
                .metadata()
                .permission_mode()
                .expect("entry should have permission mode metadata")
                .get()
                & 0o777,
        ));
    })
    .unwrap();
    assert_eq!(
        entries,
        [
            ("outer/before.txt".to_string(), 0o666),
            ("inner/a.txt".to_string(), 0o666),
            ("inner/b.txt".to_string(), 0o666),
            ("outer/after.txt".to_string(), 0o666),
        ]
    );
}
