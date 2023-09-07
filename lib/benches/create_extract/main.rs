#![feature(test)]
extern crate test;

use libpna::{ArchiveReader, ArchiveWriter, EntryBuilder, ReadOption, WriteOptionBuilder};
use std::io::{Cursor, Read, Write};
use test::Bencher;

mod archive;
mod empty;

fn bench_write_archive(b: &mut Bencher, mut options: WriteOptionBuilder) {
    b.iter(|| {
        let mut vec = Vec::with_capacity(10000);
        let mut writer = ArchiveWriter::write_header(&mut vec).unwrap();
        writer
            .add_entry({
                let mut builder = EntryBuilder::new_file(
                    "bench".try_into().unwrap(),
                    options.password(Some("password")).build(),
                )
                .unwrap();
                builder.write_all(&vec![24; 1111]).unwrap();
                builder.build().unwrap()
            })
            .unwrap();
        writer.finalize().unwrap();
    })
}

fn bench_read_archive(b: &mut Bencher, mut options: WriteOptionBuilder) {
    let mut writer = ArchiveWriter::write_header(Vec::with_capacity(10000)).unwrap();
    writer
        .add_entry({
            let mut builder = EntryBuilder::new_file(
                "bench".try_into().unwrap(),
                options.password(Some("password")).build(),
            )
            .unwrap();
            builder.write_all(&vec![24; 1111]).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    let vec = writer.finalize().unwrap();

    b.iter(|| {
        let mut reader = ArchiveReader::read_header(Cursor::new(vec.as_slice())).unwrap();
        for item in reader.entries() {
            let mut buf = Vec::with_capacity(1000);
            item.unwrap()
                .into_reader(ReadOption::builder().password("password").build())
                .unwrap()
                .read_to_end(&mut buf)
                .unwrap();
        }
    })
}
