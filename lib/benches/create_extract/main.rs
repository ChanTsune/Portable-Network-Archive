#![feature(test)]
extern crate test;

use libpna::{
    ArchiveReader, Encoder, EntryBuilder, ReadEntry, ReadOptionBuilder, WriteOptionBuilder,
};
use std::io::{Cursor, Read, Write};
use test::Bencher;

mod archive;
mod empty;

fn bench_write_archive(b: &mut Bencher, mut options: WriteOptionBuilder) {
    b.iter(|| {
        let mut vec = Vec::with_capacity(10000);
        let encoder = Encoder::default();
        let mut writer = encoder.write_header(&mut vec).unwrap();
        writer
            .add_entry({
                let mut builder = EntryBuilder::new_file(
                    "bench".into(),
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
    let mut vec = Vec::with_capacity(10000);
    {
        let encoder = Encoder::default();
        let mut writer = encoder.write_header(&mut vec).unwrap();
        writer
            .add_entry({
                let mut builder = EntryBuilder::new_file(
                    "bench".into(),
                    options.password(Some("password")).build(),
                )
                .unwrap();
                builder.write_all(&vec![24; 1111]).unwrap();
                builder.build().unwrap()
            })
            .unwrap();
        writer.finalize().unwrap();
    }

    b.iter(|| {
        let mut reader = ArchiveReader::read_header(Cursor::new(vec.as_slice())).unwrap();
        while let Some(item) = reader.read().unwrap() {
            let mut buf = Vec::with_capacity(1000);
            item.into_reader(ReadOptionBuilder::new().password("password").build())
                .unwrap()
                .read_to_end(&mut buf)
                .unwrap();
        }
    })
}
