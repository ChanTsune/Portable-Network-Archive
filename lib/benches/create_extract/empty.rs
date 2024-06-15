extern crate test;

use libpna::{Archive, ReadOptions};
use std::io;
use test::Bencher;

#[bench]
fn write_empty_archive(b: &mut Bencher) {
    b.iter(|| {
        let mut vec = Vec::with_capacity(1000);
        let writer = Archive::write_header(&mut vec).expect("failed to write header");
        writer.finalize().expect("failed to finalize");
    })
}

#[bench]
fn read_empty_archive(b: &mut Bencher) {
    let writer = Archive::write_header(Vec::with_capacity(1000)).expect("failed to write header");
    let vec = writer.finalize().expect("failed to finalize");

    b.iter(|| {
        let mut reader = Archive::read_header(vec.as_slice()).expect("failed to read header");
        for entry in reader.entries_skip_solid() {
            let item = entry.expect("failed to read entry");
            io::read_to_string(
                item.reader(ReadOptions::builder().build())
                    .expect("failed to read entry"),
            )
            .expect("failed to make string");
        }
    })
}
