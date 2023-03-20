extern crate test;

use libpna::{ArchiveReader, Encoder, ReadEntry, ReadOptionBuilder};
use std::io::{self, Cursor};
use test::Bencher;

#[bench]
fn write_empty_archive(b: &mut Bencher) {
    b.iter(|| {
        let mut vec = Vec::with_capacity(1000);
        let encoder = Encoder::default();
        let mut writer = encoder.write_header(&mut vec).unwrap();
        writer.finalize().unwrap();
    })
}

#[bench]
fn read_empty_archive(b: &mut Bencher) {
    let encoder = Encoder::default();
    let mut writer = encoder.write_header(Vec::with_capacity(1000)).unwrap();
    let mut vec = writer.finalize().unwrap();

    b.iter(|| {
        let mut reader = ArchiveReader::read_header(Cursor::new(vec.as_slice())).unwrap();
        for entry in reader.entries() {
            let item = entry.unwrap();
            io::read_to_string(item.into_reader(ReadOptionBuilder::new().build()).unwrap())
                .unwrap();
        }
    })
}
