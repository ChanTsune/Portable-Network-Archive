use libpna::{Archive, DataKind, ReadOptions};
use std::{collections::BTreeSet, io};

/// Whether a fixture's file entries carry the optional fSIZ chunk.
#[derive(Clone, Copy)]
enum RawFileSize {
    Stored,
    Absent,
}

/// Reads every entry of a fixture and compares it against the original file.
///
/// Counts what it checked and collects the exact path set: without that, a
/// fixture that lost or renamed an entry would leave that arm's assertions
/// unexecuted and the test would still pass.
fn extract_all(bytes: &[u8], password: Option<&[u8]>, raw_file_size: RawFileSize) {
    let mut n = 0;
    let mut paths = BTreeSet::new();
    let mut archive_reader = Archive::read_header(bytes).unwrap();
    for entry in archive_reader.entries().skip_solid() {
        let item = entry.unwrap();
        if item.header().data_kind() == DataKind::DIRECTORY {
            continue;
        }
        n += 1;
        let path = item.header().path().as_str();
        paths.insert(path.to_string());
        let mut dist = Vec::new();
        let mut reader = item.reader(ReadOptions::with_password(password)).unwrap();
        io::copy(&mut reader, &mut dist).unwrap();
        let bytes: &[u8] = match path {
            "raw/first/second/third/pna.txt" => {
                include_bytes!("../../resources/test/raw/first/second/third/pna.txt")
            }
            "raw/images/icon.bmp" => include_bytes!("../../resources/test/raw/images/icon.bmp"),
            "raw/images/icon.png" => include_bytes!("../../resources/test/raw/images/icon.png"),
            "raw/images/icon.svg" => include_bytes!("../../resources/test/raw/images/icon.svg"),
            "raw/parent/child.txt" => include_bytes!("../../resources/test/raw/parent/child.txt"),
            "raw/pna/empty.pna" => include_bytes!("../../resources/test/raw/pna/empty.pna"),
            "raw/pna/nest.pna" => include_bytes!("../../resources/test/raw/pna/nest.pna"),
            "raw/empty.txt" => include_bytes!("../../resources/test/raw/empty.txt"),
            "raw/text.txt" => include_bytes!("../../resources/test/raw/text.txt"),
            a => panic!("Unexpected entry name {a}"),
        };
        assert_eq!(dist.as_slice(), bytes);
        let expected = match raw_file_size {
            RawFileSize::Stored => Some(bytes.len() as u128),
            RawFileSize::Absent => None,
        };
        assert_eq!(item.metadata().raw_file_size(), expected, "{path}");
    }
    assert_eq!(
        paths,
        BTreeSet::from(
            [
                "raw/first/second/third/pna.txt",
                "raw/images/icon.bmp",
                "raw/images/icon.png",
                "raw/images/icon.svg",
                "raw/parent/child.txt",
                "raw/pna/empty.pna",
                "raw/pna/nest.pna",
                "raw/empty.txt",
                "raw/text.txt",
            ]
            .map(String::from)
        )
    );
    assert_eq!(n, 9);
}

/// The empty fixture holds no entries, so it cannot go through `extract_all`:
/// asserting that it yields nothing is the whole of what this fixture pins.
#[test]
fn empty() {
    let bytes = include_bytes!("../../resources/test/empty.pna");
    let mut archive_reader = Archive::read_header(bytes.as_slice()).unwrap();
    assert!(archive_reader.entries().skip_solid().next().is_none());
}

#[test]
fn store() {
    extract_all(
        include_bytes!("../../resources/test/store.pna"),
        None,
        RawFileSize::Absent,
    );
}

#[test]
fn deflate() {
    extract_all(
        include_bytes!("../../resources/test/deflate.pna"),
        None,
        RawFileSize::Absent,
    );
}

#[test]
fn zstd() {
    extract_all(
        include_bytes!("../../resources/test/zstd.pna"),
        None,
        RawFileSize::Absent,
    );
}

#[test]
fn xz() {
    extract_all(
        include_bytes!("../../resources/test/xz.pna"),
        None,
        RawFileSize::Absent,
    );
}

#[test]
fn zstd_aes_cbc() {
    extract_all(
        include_bytes!("../../resources/test/zstd_aes_cbc.pna"),
        Some(b"password"),
        RawFileSize::Absent,
    );
}

#[test]
fn zstd_aes_ctr() {
    extract_all(
        include_bytes!("../../resources/test/zstd_aes_ctr.pna"),
        Some(b"password"),
        RawFileSize::Absent,
    );
}

#[test]
fn zstd_camellia_cbc() {
    extract_all(
        include_bytes!("../../resources/test/zstd_camellia_cbc.pna"),
        Some(b"password"),
        RawFileSize::Absent,
    );
}

#[test]
fn zstd_camellia_ctr() {
    extract_all(
        include_bytes!("../../resources/test/zstd_camellia_ctr.pna"),
        Some(b"password"),
        RawFileSize::Absent,
    );
}

#[test]
fn zstd_aes_gcm() {
    extract_all(
        include_bytes!("../../resources/test/zstd_aes_gcm.pna"),
        Some(b"password"),
        RawFileSize::Stored,
    );
}

#[test]
fn zstd_camellia_gcm() {
    extract_all(
        include_bytes!("../../resources/test/zstd_camellia_gcm.pna"),
        Some(b"password"),
        RawFileSize::Stored,
    );
}

#[test]
fn keep_permission() {
    extract_all(
        include_bytes!("../../resources/test/zstd_keep_permission.pna"),
        None,
        RawFileSize::Absent,
    );
}

#[test]
fn keep_timestamp() {
    extract_all(
        include_bytes!("../../resources/test/zstd_keep_timestamp.pna"),
        None,
        RawFileSize::Absent,
    );
}

#[test]
fn keep_timestamp_with_nanos() {
    extract_all(
        include_bytes!("../../resources/test/zstd_keep_timestamp_with_nanos.pna"),
        None,
        RawFileSize::Stored,
    );
}

#[test]
fn keep_xattr() {
    extract_all(
        include_bytes!("../../resources/test/zstd_keep_xattr.pna"),
        None,
        RawFileSize::Stored,
    );
}

#[test]
fn keep_dir() {
    extract_all(
        include_bytes!("../../resources/test/zstd_keep_dir.pna"),
        None,
        RawFileSize::Absent,
    );
}

#[test]
fn zstd_with_raw_file_size() {
    extract_all(
        include_bytes!("../../resources/test/zstd_with_raw_file_size.pna"),
        None,
        RawFileSize::Stored,
    );
}
