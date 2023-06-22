use libpna::{
    ArchiveReader, ArchiveWriter, Compression, EntryBuilder, ReadEntry, ReadOption, WriteOption,
};
use std::io::{self, Read, Write};

/// Change the entries compression method
fn change_compression_method<R: Read, W: Write>(
    r: R,
    w: W,
    compression: Compression,
) -> io::Result<()> {
    let mut writer = ArchiveWriter::write_header(w)?;
    let mut reader = ArchiveReader::read_header(r)?;
    for entry in reader.entries() {
        let entry = entry?;
        let header = entry.header();
        let mut builder = EntryBuilder::new_file(
            header.path().clone(),
            WriteOption::builder().compression(compression).build(),
        )?;
        let mut reader = entry.into_reader(ReadOption::builder().build())?;
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
