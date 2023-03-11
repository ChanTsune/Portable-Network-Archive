use libpna::{Decoder, Entry, ReadOptionBuilder};
use std::io;

fn extract_all(bytes: &[u8], password: Option<&str>) {
    let decoder = Decoder::new();
    let mut archive_reader = decoder.read_header(io::Cursor::new(bytes)).unwrap();
    while let Some(item) = archive_reader.read().unwrap() {
        let path = item.header().path().to_string();
        let mut dist = Vec::new();
        let mut reader = item
            .into_reader({
                let mut builder = ReadOptionBuilder::new();
                if let Some(password) = password {
                    builder.password(password);
                }
                builder.build()
            })
            .unwrap();
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
