#![feature(test)]
extern crate test;

use libpna::{Decoder, Encoder};
use std::io::{self, Cursor};
use test::Bencher;

#[bench]
fn write_store_archive(b: &mut Bencher) {
    b.iter(|| {
        let mut vec = Vec::with_capacity(100000);
        let encoder = Encoder::default();
        let mut writer = encoder.write_header(&mut vec).unwrap();
        for i in 0..100 {
            writer.start_file(&format!("{i}")).unwrap();
            writer.write_all(&vec![i as u8; i * i]).unwrap();
            writer.end_file().unwrap();
        }
        writer.finalize().unwrap();
    })
}

#[bench]
fn read_store_archive(b: &mut Bencher) {
    let mut vec = Vec::with_capacity(100000);
    {
        let encoder = Encoder::default();
        let mut writer = encoder.write_header(&mut vec).unwrap();
        for i in 0..100 {
            writer.start_file(&format!("{i}")).unwrap();
            writer.write_all(&vec![i as u8; i * i]).unwrap();
            writer.end_file().unwrap();
        }
        writer.finalize().unwrap();
    }

    b.iter(|| {
        let decoder = Decoder::default();
        let mut reader = decoder.read_header(Cursor::new(vec.as_slice())).unwrap();
        while let Some(mut item) = reader.read(None).unwrap() {
            io::read_to_string(item).unwrap();
        }
    })
}
