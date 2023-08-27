use libpna::{ArchiveReader, DataKind, ReadOption};
use std::io;

fn extract_all(bytes: &[u8], password: Option<&str>) {
    let mut archive_reader = ArchiveReader::read_header(io::Cursor::new(bytes)).unwrap();
    for entry in archive_reader.entries_with_password(password.map(String::from)) {
        let item = entry.unwrap();
        if item.header().data_kind() == DataKind::Directory {
            continue;
        }
        let path = item.header().path().to_string();
        let mut dist = Vec::new();
        let mut reader = item
            .into_reader({
                let mut builder = ReadOption::builder();
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
fn solid() {
    extract_all(include_bytes!("../../resources/test/solid.pna"), None);
}
