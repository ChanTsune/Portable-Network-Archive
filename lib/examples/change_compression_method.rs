use libpna::{Archive, Compression, EntryBuilder, ReadOptions, WriteOptions};
use std::io::{self, Read, Write};

/// Change the entries compression method
fn change_compression_method<R: Read, W: Write>(
    r: R,
    w: W,
    compression: Compression,
) -> io::Result<()> {
    let mut writer = Archive::write_header(w)?;
    let mut reader = Archive::read_header(r)?;
    for entry in reader.entries().skip_solid() {
        let entry = entry?;
        let header = entry.header();
        let mut builder = EntryBuilder::new_file(
            header.path().clone(),
            WriteOptions::builder().compression(compression).build(),
        )?;
        let mut reader = entry.reader(ReadOptions::builder().build())?;
        io::copy(&mut reader, &mut builder)?;
        writer.add_entry(builder.build()?)?;
    }
    writer.finalize()?;
    Ok(())
}

fn main() -> io::Result<()> {
    let mut dist = vec![];
    change_compression_method(
        include_bytes!("../../resources/test/deflate.pna").as_slice(),
        &mut dist,
        Compression::Deflate,
    )
}
