use libpna::{Archive, DataKind, NormalEntry, ReadEntry, ReadOptions};
use std::io;

fn assert_entry(item: NormalEntry, password: Option<&[u8]>) {
    let path = item.header().path().as_str();
    let mut dist = Vec::new();
    let mut reader = item.reader(ReadOptions::with_password(password)).unwrap();
    io::copy(&mut reader, &mut dist).unwrap();
    match path {
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

fn extract_all(bytes: &[u8], password: Option<&[u8]>) {
    let mut n = 0;
    let mut archive_reader = Archive::read_header(bytes).unwrap();
    for entry in archive_reader.entries_with_password(password) {
        let item = entry.unwrap();
        if item.header().data_kind() == DataKind::Directory {
            continue;
        }
        n += 1;
        assert_entry(item, password);
    }
    assert_eq!(n, 9);

    let mut n = 0;
    let mut archive_reader = Archive::read_header(bytes).unwrap();
    for entry in archive_reader.entries() {
        let item = entry.unwrap();
        match item {
            ReadEntry::Solid(item) => {
                for item in item.entries(password).unwrap() {
                    let item = item.unwrap();
                    if item.header().data_kind() == DataKind::Directory {
                        continue;
                    }
                    n += 1;
                    assert_entry(item, password);
                }
            }
            ReadEntry::Normal(item) => {
                if item.header().data_kind() == DataKind::Directory {
                    continue;
                }
                n += 1;
                assert_entry(item, password);
            }
        }
    }
    assert_eq!(n, 9);
}

#[test]
fn solid_store() {
    extract_all(include_bytes!("../../resources/test/solid_store.pna"), None);
}

#[test]
fn solid_zstd() {
    extract_all(include_bytes!("../../resources/test/solid_zstd.pna"), None);
}

#[test]
fn solid_xz() {
    extract_all(include_bytes!("../../resources/test/solid_xz.pna"), None);
}

#[test]
fn solid_deflate() {
    extract_all(
        include_bytes!("../../resources/test/solid_deflate.pna"),
        None,
    );
}
