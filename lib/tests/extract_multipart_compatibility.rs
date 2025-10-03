use libpna::{Archive, DataKind, ReadOptions};
use std::io;

fn extract_all(follows: &[&[u8]], password: Option<&str>) {
    let mut idx = 0;
    let mut archive_reader = Archive::read_header(follows[idx]).unwrap();
    loop {
        idx += 1;
        for entry in archive_reader.entries().skip_solid() {
            let item = entry.unwrap();
            if item.header().data_kind() == DataKind::Directory {
                continue;
            }
            let path = item.header().path().to_string();
            let mut dist = Vec::new();
            let mut reader = item.reader(ReadOptions::with_password(password)).unwrap();
            io::copy(&mut reader, &mut dist).unwrap();
            match &*path {
                "multipart_test.txt" => assert_eq!(
                    dist.as_slice(),
                    include_bytes!("../../resources/test/multipart_test.txt")
                ),
                a => panic!("Unexpected entry name {a}"),
            }
        }
        if archive_reader.has_next_archive() {
            archive_reader = archive_reader.read_next_archive(follows[idx]).unwrap();
        } else {
            break;
        }
    }
}

#[test]
fn extract_multipart_archive() {
    extract_all(
        &[
            include_bytes!("../../resources/test/multipart.part1.pna"),
            include_bytes!("../../resources/test/multipart.part2.pna"),
        ],
        None,
    );
}
