use crate::utils::{archive, archive::FileEntryDef, setup};
use clap::Parser;
use pna::prelude::*;
use portable_network_archive::cli;
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

/// Precondition: A solid archive contains entries in non-alphabetical order.
/// Action: Run `pna sort` without `--unsolid`.
/// Expectation: Entries are sorted inside the preserved solid block.
#[test]
fn sort_solid_archive_without_unsolid() {
    setup();
    let path = "sort_solid_without_unsolid.pna";
    archive::create_solid_archive_with_permissions(
        path,
        &[
            FileEntryDef {
                path: "b.txt",
                content: b"b",
                permission: 0o644,
            },
            FileEntryDef {
                path: "a.txt",
                content: b"a",
                permission: 0o644,
            },
        ],
    )
    .unwrap();
    cli::Cli::try_parse_from(["pna", "--quiet", "sort", "-f", path])
        .unwrap()
        .execute()
        .unwrap();

    let mut archive = pna::Archive::open(path).unwrap();
    let entries = archive.entries().collect::<Result<Vec<_>, _>>().unwrap();
    let [pna::ReadEntry::Solid(solid)] = entries.as_slice() else {
        panic!("expected exactly one solid block");
    };
    let names = solid
        .entries(&pna::ReadOptions::with_password::<&str>(None))
        .unwrap()
        .map(|entry| entry.unwrap().name().to_string())
        .collect::<Vec<_>>();
    assert_eq!(names, ["a.txt", "b.txt"]);
}

fn stored_file_entry(path: &str) -> pna::NormalEntry {
    let mut builder =
        pna::FileEntryBuilder::new_with_options(path.into(), pna::WriteOptions::store()).unwrap();
    builder.write_all(path.as_bytes()).unwrap();
    builder.build().unwrap()
}

fn layout(path: &str) -> Vec<(bool, Vec<String>)> {
    pna::Archive::open(path)
        .unwrap()
        .entries()
        .map(|entry| match entry.unwrap() {
            pna::ReadEntry::Normal(entry) => (false, vec![entry.name().to_string()]),
            pna::ReadEntry::Solid(entry) => (
                true,
                entry
                    .entries(&pna::ReadOptions::with_password::<&str>(None))
                    .unwrap()
                    .map(|entry| entry.unwrap().name().to_string())
                    .collect(),
            ),
        })
        .collect()
}

/// Precondition: Normal runs surround multiple solid blocks, including adjacent empty blocks.
/// Action: Run `pna sort` with `--keep-solid`, then sort by descending name.
/// Expectation: Each run and solid block is sorted independently without crossing boundaries.
#[test]
fn sort_keep_solid_mixed_archive() {
    setup();
    let path = "sort_keep_solid_mixed.pna";
    let file = fs::File::create(path).unwrap();
    let mut out = pna::Archive::write_header(file).unwrap();
    out.add_entry(stored_file_entry("c.txt")).unwrap();
    out.add_entry(stored_file_entry("b.txt")).unwrap();
    let mut solid = pna::SolidEntryBuilder::new(pna::WriteOptions::store()).unwrap();
    solid.add_entry(stored_file_entry("f.txt")).unwrap();
    solid.add_entry(stored_file_entry("d.txt")).unwrap();
    out.add_entry(solid.build().unwrap()).unwrap();
    out.add_entry(stored_file_entry("z.txt")).unwrap();
    out.add_entry(stored_file_entry("a.txt")).unwrap();
    let mut solid = pna::SolidEntryBuilder::new(pna::WriteOptions::store()).unwrap();
    solid.add_entry(stored_file_entry("y.txt")).unwrap();
    solid.add_entry(stored_file_entry("x.txt")).unwrap();
    out.add_entry(solid.build().unwrap()).unwrap();
    let solid = pna::SolidEntryBuilder::new(pna::WriteOptions::store()).unwrap();
    out.add_entry(solid.build().unwrap()).unwrap();
    out.add_entry(stored_file_entry("e.txt")).unwrap();
    out.add_entry(stored_file_entry("d.txt")).unwrap();
    out.finalize().unwrap();

    cli::Cli::try_parse_from(["pna", "--quiet", "sort", "--keep-solid", "-f", path])
        .unwrap()
        .execute()
        .unwrap();

    assert_eq!(
        layout(path),
        [
            (false, vec!["b.txt".into()]),
            (false, vec!["c.txt".into()]),
            (true, vec!["d.txt".into(), "f.txt".into()]),
            (false, vec!["a.txt".into()]),
            (false, vec!["z.txt".into()]),
            (true, vec!["x.txt".into(), "y.txt".into()]),
            (true, vec![]),
            (false, vec!["d.txt".into()]),
            (false, vec!["e.txt".into()]),
        ]
    );

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "sort",
        "--keep-solid",
        "-f",
        path,
        "--by",
        "name:desc",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        layout(path),
        [
            (false, vec!["c.txt".into()]),
            (false, vec!["b.txt".into()]),
            (true, vec!["f.txt".into(), "d.txt".into()]),
            (false, vec!["z.txt".into()]),
            (false, vec!["a.txt".into()]),
            (true, vec!["y.txt".into(), "x.txt".into()]),
            (true, vec![]),
            (false, vec!["e.txt".into()]),
            (false, vec!["d.txt".into()]),
        ]
    );
}

/// Precondition: An encrypted, compressed solid block contains unsorted files and extra chunks.
/// Action: Run `pna sort` by descending modification time with `--keep-solid` and the correct password.
/// Expectation: The sorted block retains compression, encryption, extra chunks, metadata and contents.
#[test]
fn sort_keep_solid_encrypted_solid_archive() {
    setup();
    let path = "sort_keep_solid_encrypted.pna";
    let options = pna::WriteOptions::builder()
        .compression(pna::Compression::ZSTANDARD)
        .encryption(pna::Encryption::AES)
        .cipher_mode(pna::CipherMode::GCM)
        .password(Some("password"))
        .build();
    let chunk = pna::RawChunk::from_data(
        pna::ChunkType::private(*b"soRt").unwrap(),
        b"block metadata".to_vec(),
    );
    let mut solid = pna::SolidEntryBuilder::new(options).unwrap();
    solid.add_extra_chunk(chunk.clone());
    solid
        .add_entry(
            stored_file_entry("a.txt").with_metadata(
                pna::Metadata::new().with_modified(Some(pna::Duration::seconds(1234))),
            ),
        )
        .unwrap();
    solid
        .add_entry(
            stored_file_entry("b.txt").with_metadata(
                pna::Metadata::new().with_modified(Some(pna::Duration::seconds(5678))),
            ),
        )
        .unwrap();
    let mut out = pna::Archive::write_header(fs::File::create(path).unwrap()).unwrap();
    out.add_entry(solid.build().unwrap()).unwrap();
    out.finalize().unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "sort",
        "--keep-solid",
        "-f",
        path,
        "--password",
        "password",
        "--by",
        "mtime:desc",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let entries = pna::Archive::open(path)
        .unwrap()
        .entries()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let [pna::ReadEntry::Solid(solid)] = entries.as_slice() else {
        panic!("expected exactly one encrypted solid block");
    };
    assert_eq!(solid.compression(), pna::Compression::ZSTANDARD);
    assert_eq!(solid.encryption(), pna::Encryption::AES);
    assert_eq!(solid.cipher_mode(), pna::CipherMode::GCM);
    assert_eq!(solid.extra_chunks(), [chunk]);
    let options = pna::ReadOptions::with_password(Some("password"));
    let entries = solid
        .entries(&options)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let mut content = Vec::new();
            entry
                .reader(&options)
                .unwrap()
                .read_to_end(&mut content)
                .unwrap();
            (
                entry.name().to_string(),
                entry.metadata().modified(),
                content,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        entries,
        [
            (
                "b.txt".into(),
                Some(pna::Duration::seconds(5678)),
                b"b.txt".to_vec()
            ),
            (
                "a.txt".into(),
                Some(pna::Duration::seconds(1234)),
                b"a.txt".to_vec()
            ),
        ]
    );
}

/// Precondition: A non-solid encrypted archive contains entries in non-alphabetical order.
/// Action: Run `pna sort` with the correct password and no solid strategy option.
/// Expectation: Entries are sorted and every entry remains encrypted with the same
///              password.
#[test]
fn sort_encrypted_archive_keeps_encryption() {
    setup();
    let path = "sort_encrypted_keeps_encryption.pna";
    archive::create_encrypted_archive_with_permissions(
        path,
        &[
            FileEntryDef {
                path: "b.txt",
                content: b"b",
                permission: 0o644,
            },
            FileEntryDef {
                path: "a.txt",
                content: b"a",
                permission: 0o644,
            },
        ],
        "password",
    )
    .unwrap();

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "sort",
        "-f",
        path,
        "--password",
        "password",
    ])
    .unwrap()
    .execute()
    .unwrap();

    let mut entries = Vec::new();
    archive::for_each_entry_with_password(path, "password", |entry| {
        let mut content = Vec::new();
        entry
            .reader(pna::ReadOptions::with_password(Some("password")))
            .unwrap()
            .read_to_end(&mut content)
            .unwrap();
        entries.push((
            entry.header().path().to_string(),
            entry.header().encryption(),
            entry.header().cipher_mode(),
            content,
        ));
    })
    .unwrap();
    assert_eq!(
        entries,
        [
            (
                "a.txt".to_string(),
                pna::Encryption::AES,
                pna::CipherMode::CTR,
                b"a".to_vec(),
            ),
            (
                "b.txt".to_string(),
                pna::Encryption::AES,
                pna::CipherMode::CTR,
                b"b".to_vec(),
            ),
        ]
    );
}

/// Precondition: An encrypted solid archive and a pre-existing output file exist.
/// Action: Run `pna sort` with `--output`, `--overwrite` and an incorrect password.
/// Expectation: The command fails and both the archive and the output file remain
///              byte-for-byte unchanged.
#[test]
fn sort_solid_archive_wrong_password_preserves_output() {
    setup();
    let path = "sort_solid_output_preserved.pna";
    archive::create_encrypted_solid_archive_with_permissions(
        path,
        &[
            FileEntryDef {
                path: "b.txt",
                content: b"b",
                permission: 0o644,
            },
            FileEntryDef {
                path: "a.txt",
                content: b"a",
                permission: 0o644,
            },
        ],
        "password",
    )
    .unwrap();
    let original = fs::read(path).unwrap();
    fs::write("sort_solid_output_preserved_out.pna", b"sentinel").unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "sort",
        "--password",
        "incorrect",
        "-f",
        path,
        "--output",
        "sort_solid_output_preserved_out.pna",
        "--overwrite",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert_eq!(fs::read(path).unwrap(), original);
    assert_eq!(
        fs::read("sort_solid_output_preserved_out.pna").unwrap(),
        b"sentinel"
    );
}

/// Precondition: An encrypted solid archive exists and the output path does not.
/// Action: Run `pna sort` with `--output` and an incorrect password.
/// Expectation: The command fails and no output file is created.
#[test]
fn sort_solid_archive_wrong_password_creates_no_output() {
    setup();
    let path = "sort_solid_output_not_created.pna";
    archive::create_encrypted_solid_archive_with_permissions(
        path,
        &[
            FileEntryDef {
                path: "b.txt",
                content: b"b",
                permission: 0o644,
            },
            FileEntryDef {
                path: "a.txt",
                content: b"a",
                permission: 0o644,
            },
        ],
        "password",
    )
    .unwrap();

    let result = cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "sort",
        "--password",
        "incorrect",
        "-f",
        path,
        "--output",
        "sort_solid_output_not_created_out.pna",
    ])
    .unwrap()
    .execute();

    assert!(result.is_err());
    assert!(!Path::new("sort_solid_output_not_created_out.pna").exists());
}

pub(crate) fn create_split_solid_archive(dir: &str) -> Vec<Vec<u8>> {
    fs::create_dir_all(format!("{dir}/in")).unwrap();
    fs::write(format!("{dir}/in/b.txt"), "b".repeat(200)).unwrap();
    fs::write(format!("{dir}/in/a.txt"), "a".repeat(200)).unwrap();
    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "c",
        "-f",
        &format!("{dir}/archive.pna"),
        "--overwrite",
        "--solid",
        "--store",
        "--unstable",
        "--split",
        "300",
        &format!("{dir}/in/b.txt"),
        &format!("{dir}/in/a.txt"),
    ])
    .unwrap()
    .execute()
    .unwrap();
    let mut parts = Vec::new();
    while let Ok(bytes) = fs::read(format!("{dir}/archive.part{}.pna", parts.len() + 1)) {
        parts.push(bytes);
    }
    assert!(parts.len() > 1, "expected the archive to be split");
    parts
}

/// Precondition: A solid archive is split into several parts.
/// Action: Run `pna sort` with `--output` on the first part without `--unsolid`.
/// Expectation: The output retains a sorted solid block and every source part is unchanged.
#[test]
fn sort_split_solid_archive_without_unsolid() {
    setup();
    let dir = "sort_split_solid_without_unsolid";
    let parts = create_split_solid_archive(dir);
    let output = format!("{dir}/sorted.pna");

    cli::Cli::try_parse_from([
        "pna",
        "--quiet",
        "sort",
        "-f",
        &format!("{dir}/archive.part1.pna"),
        "--output",
        &output,
        "--overwrite",
    ])
    .unwrap()
    .execute()
    .unwrap();

    assert_eq!(
        layout(&output),
        [(
            true,
            vec![format!("{dir}/in/a.txt"), format!("{dir}/in/b.txt"),]
        )]
    );
    let mut after = Vec::new();
    while let Ok(bytes) = fs::read(format!("{dir}/archive.part{}.pna", after.len() + 1)) {
        after.push(bytes);
    }
    assert_eq!(after, parts);
}

/// Precondition: A non-solid archive contains entries in non-alphabetical order.
/// Action: Run `pna sort` with `--keep-solid`.
/// Expectation: Entries are sorted by name.
#[test]
fn sort_non_solid_archive_with_keep_solid() {
    setup();
    let path = "sort_non_solid_with_keep_solid.pna";
    archive::create_archive_with_permissions(
        path,
        &[
            FileEntryDef {
                path: "b.txt",
                content: b"b",
                permission: 0o644,
            },
            FileEntryDef {
                path: "a.txt",
                content: b"a",
                permission: 0o644,
            },
        ],
    )
    .unwrap();

    cli::Cli::try_parse_from(["pna", "--quiet", "sort", "--keep-solid", "-f", path])
        .unwrap()
        .execute()
        .unwrap();

    assert_eq!(archive::get_archive_entry_names(path), ["a.txt", "b.txt"]);
}
