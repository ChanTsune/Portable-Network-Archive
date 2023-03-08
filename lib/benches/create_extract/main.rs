#![feature(test)]
mod archive;
mod empty;

#[macro_export]
macro_rules! bench_write_archive {
    ($name:ident, $options:expr) => {
        #[bench]
        fn $name(b: &mut Bencher) {
            b.iter(|| {
                let mut vec = Vec::with_capacity(100000);
                let encoder = Encoder::default();
                let mut writer = encoder.write_header(&mut vec).unwrap();
                for i in 0i32..10 {
                    writer
                        .start_file_with_options(
                            (&format!("{i}")).into(),
                            $options.password(Some("password".to_string())),
                        )
                        .unwrap();
                    writer.write_all(&vec![i as u8; i.pow(4) as usize]).unwrap();
                    writer.end_file().unwrap();
                }
                writer.finalize().unwrap();
            })
        }
    };
}

#[macro_export]
macro_rules! bench_read_archive {
    ($name:ident, $options:expr) => {
        #[bench]
        fn $name(b: &mut Bencher) {
            let mut vec = Vec::with_capacity(100000);
            {
                let encoder = Encoder::default();
                let mut writer = encoder.write_header(&mut vec).unwrap();
                for i in 0i32..10 {
                    writer
                        .start_file_with_options(
                            (&format!("{i}")).into(),
                            $options.password(Some("password".to_string())),
                        )
                        .unwrap();
                    writer.write_all(&vec![i as u8; i.pow(4) as usize]).unwrap();
                    writer.end_file().unwrap();
                }
                writer.finalize().unwrap();
            }

            b.iter(|| {
                let decoder = Decoder::default();
                let mut reader = decoder.read_header(Cursor::new(vec.as_slice())).unwrap();
                while let Some(item) = reader.read(Some("password")).unwrap() {
                    let mut buf = Vec::with_capacity(1000);
                    item.reader().read_to_end(&mut buf).unwrap();
                }
            })
        }
    };
}
