#![feature(test)]
extern crate test;

use libpna::{Archive, EntryBuilder, ReadOption, WriteOptionBuilder};
use std::io::{Read, Write};
use test::Bencher;

mod archive;
mod empty;

fn bench_write_archive(b: &mut Bencher, mut options: WriteOptionBuilder) {
    let buf = [24; 1111];
    b.iter(|| {
        let mut vec = Vec::with_capacity(10000);
        let mut writer = Archive::write_header(&mut vec).unwrap();
        writer
            .add_entry({
                let mut builder = EntryBuilder::new_file(
                    "bench".into(),
                    options.password(Some("password")).build(),
                )
                .unwrap();
                builder.write_all(&buf).unwrap();
                builder.build().unwrap()
            })
            .unwrap();
        writer.finalize().unwrap();
    })
}

fn bench_read_archive(b: &mut Bencher, mut options: WriteOptionBuilder) {
    let buf = [24; 1111];
    let mut writer = Archive::write_header(Vec::with_capacity(10000)).unwrap();
    writer
        .add_entry({
            let mut builder =
                EntryBuilder::new_file("bench".into(), options.password(Some("password")).build())
                    .unwrap();
            builder.write_all(&buf).unwrap();
            builder.build().unwrap()
        })
        .unwrap();
    let vec = writer.finalize().unwrap();

    b.iter(|| {
        let mut reader = Archive::read_header(vec.as_slice()).unwrap();
        for item in reader.entries_skip_solid() {
            let mut buf = Vec::with_capacity(1000);
            item.unwrap()
                .reader(ReadOption::with_password(Some("password")))
                .unwrap()
                .read_to_end(&mut buf)
                .unwrap();
        }
    })
}
