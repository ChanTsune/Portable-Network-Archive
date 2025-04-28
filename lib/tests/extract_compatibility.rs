use libpna::{Archive, DataKind, ReadOptions};
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
        let mut reader = item.reader(ReadOptions::with_password(password)).unwrap();
        io::copy(&mut reader, &mut dist).unwrap();
        match &*path {
            "raw/first/second/third/pna.txt" => {
                let bytes = include_bytes!("../../resources/test/raw/first/second/third/pna.txt");
                assert_eq!(dist.as_slice(), bytes);
                if let Some(size) = item.metadata().raw_file_size() {
                    assert_eq!(size, bytes.len() as u128);
                }
            }
            "raw/images/icon.bmp" => {
                let bytes = include_bytes!("../../resources/test/raw/images/icon.bmp");
                assert_eq!(dist.as_slice(), bytes);
                if let Some(size) = item.metadata().raw_file_size() {
                    assert_eq!(size, bytes.len() as u128);
                }
            }
            "raw/images/icon.png" => {
                let bytes = include_bytes!("../../resources/test/raw/images/icon.png");
                assert_eq!(dist.as_slice(), bytes);
                if let Some(size) = item.metadata().raw_file_size() {
                    assert_eq!(size, bytes.len() as u128);
                }
            }
            "raw/images/icon.svg" => {
                let bytes = include_bytes!("../../resources/test/raw/images/icon.svg");
                assert_eq!(dist.as_slice(), bytes);
                if let Some(size) = item.metadata().raw_file_size() {
                    assert_eq!(size, bytes.len() as u128);
                }
            }
            "raw/parent/child.txt" => {
                let bytes = include_bytes!("../../resources/test/raw/parent/child.txt");
                assert_eq!(dist.as_slice(), bytes);
                if let Some(size) = item.metadata().raw_file_size() {
                    assert_eq!(size, bytes.len() as u128);
                }
            }
            "raw/pna/empty.pna" => {
                let bytes = include_bytes!("../../resources/test/raw/pna/empty.pna");
                assert_eq!(dist.as_slice(), bytes);
                if let Some(size) = item.metadata().raw_file_size() {
                    assert_eq!(size, bytes.len() as u128);
                }
            }
            "raw/pna/nest.pna" => {
                let bytes = include_bytes!("../../resources/test/raw/pna/nest.pna");
                assert_eq!(dist.as_slice(), bytes);
                if let Some(size) = item.metadata().raw_file_size() {
                    assert_eq!(size, bytes.len() as u128);
                }
            }
            "raw/empty.txt" => {
                let bytes = include_bytes!("../../resources/test/raw/empty.txt");
                assert_eq!(dist.as_slice(), bytes);
                if let Some(size) = item.metadata().raw_file_size() {
                    assert_eq!(size, bytes.len() as u128);
                }
            }
            "raw/text.txt" => {
                let bytes = include_bytes!("../../resources/test/raw/text.txt");
                assert_eq!(dist.as_slice(), bytes);
                if let Some(size) = item.metadata().raw_file_size() {
                    assert_eq!(size, bytes.len() as u128);
                }
            }
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
fn xz() {
    extract_all(include_bytes!("../../resources/test/xz.pna"), None);
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
fn keep_xattr() {
    extract_all(
        include_bytes!("../../resources/test/zstd_keep_xattr.pna"),
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

#[test]
fn zstd_with_raw_file_size() {
    extract_all(
        include_bytes!("../../resources/test/zstd_with_raw_file_size.pna"),
        None,
    );
}
