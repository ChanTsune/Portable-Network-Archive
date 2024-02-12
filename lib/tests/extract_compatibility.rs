use libpna::{Archive, DataKind, ReadOption};
use std::io;

fn extract_all(bytes: &[u8], password: Option<&str>) {
    let mut archive_reader = Archive::read_header(bytes).unwrap();
    for entry in archive_reader.entries_skip_solid() {
        let item = entry.unwrap();
        if item.header().data_kind() == DataKind::Directory {
            continue;
        }
        let path = item.header().path().to_string();
        let mut dist = Vec::new();
        let mut reader = item.reader(ReadOption::with_password(password)).unwrap();
        io::copy(&mut reader, &mut dist).unwrap();
        match &*path {
            "raw/first/second/third/pna.txt" => assert_eq!(
                dist.as_slice(),
                include_bytes!("../../resources/test/raw/first/second/third/pna.txt")
            ),
            "raw/images/icon.bmp" => assert_eq!(
                dist.as_slice(),
                include_bytes!("../../resources/test/raw/images/icon.bmp")
            ),
            "raw/images/icon.png" => assert_eq!(
                dist.as_slice(),
                include_bytes!("../../resources/test/raw/images/icon.png")
            ),
            "raw/images/icon.svg" => assert_eq!(
                dist.as_slice(),
                include_bytes!("../../resources/test/raw/images/icon.svg")
            ),
            "raw/parent/child.txt" => assert_eq!(
                dist.as_slice(),
                include_bytes!("../../resources/test/raw/parent/child.txt")
            ),
            "raw/pna/empty.pna" => assert_eq!(
                dist.as_slice(),
                include_bytes!("../../resources/test/raw/pna/empty.pna")
            ),
            "raw/pna/nest.pna" => assert_eq!(
                dist.as_slice(),
                include_bytes!("../../resources/test/raw/pna/nest.pna")
            ),
            "raw/empty.txt" => assert_eq!(
                dist.as_slice(),
                include_bytes!("../../resources/test/raw/empty.txt")
            ),
            "raw/text.txt" => assert_eq!(
                dist.as_slice(),
                include_bytes!("../../resources/test/raw/text.txt")
            ),
            a => panic!("Unexpected entry name {a}"),
        }
    }
}

#[test]
fn empty() {
    extract_all(include_bytes!("../../resources/test/empty.pna"), None);
}

#[test]
fn store() {
    extract_all(include_bytes!("../../resources/test/store.pna"), None);
}

#[test]
fn deflate() {
    extract_all(include_bytes!("../../resources/test/deflate.pna"), None);
}

#[test]
fn zstd() {
    extract_all(include_bytes!("../../resources/test/zstd.pna"), None);
}

#[test]
fn lzma() {
    extract_all(include_bytes!("../../resources/test/lzma.pna"), None);
}

#[test]
fn zstd_aes_cbc() {
    extract_all(
        include_bytes!("../../resources/test/zstd_aes_cbc.pna"),
        Some("password"),
    );
}

#[test]
fn zstd_aes_ctr() {
    extract_all(
        include_bytes!("../../resources/test/zstd_aes_ctr.pna"),
        Some("password"),
    );
}

#[test]
fn zstd_camellia_cbc() {
    extract_all(
        include_bytes!("../../resources/test/zstd_camellia_cbc.pna"),
        Some("password"),
    );
}

#[test]
fn zstd_camellia_ctr() {
    extract_all(
        include_bytes!("../../resources/test/zstd_camellia_ctr.pna"),
        Some("password"),
    );
}

#[test]
fn keep_permission() {
    extract_all(
        include_bytes!("../../resources/test/zstd_keep_permission.pna"),
        None,
    );
}

#[test]
fn keep_timestamp() {
    extract_all(
        include_bytes!("../../resources/test/zstd_keep_timestamp.pna"),
        None,
    );
}

#[test]
fn keep_dir() {
    extract_all(
        include_bytes!("../../resources/test/zstd_keep_dir.pna"),
        None,
    );
}
