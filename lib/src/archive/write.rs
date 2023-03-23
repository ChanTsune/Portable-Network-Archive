use crate::{
    archive::{ArchiveHeader, Entry, PNA_HEADER},
    chunk::{ChunkType, ChunkWriter},
};
use std::io::{self, Write};

#[derive(Default)]
pub struct Encoder;

impl Encoder {
    pub fn new() -> Self {
        Self
    }

    pub fn write_header<W: Write>(&self, write: W) -> io::Result<ArchiveWriter<W>> {
        ArchiveWriter::write_header(write)
    }
}

pub struct ArchiveWriter<W: Write> {
    w: W,
    archive_number: u32,
}

impl<W: Write> ArchiveWriter<W> {
    pub fn write_header(write: W) -> io::Result<Self> {
        Self::write_header_with_archive_number(write, 0)
    }

    fn write_header_with_archive_number(mut write: W, archive_number: u32) -> io::Result<Self> {
        write.write_all(PNA_HEADER)?;
        let mut chunk_writer = ChunkWriter::from(write);
        chunk_writer.write_chunk(
            ChunkType::AHED,
            &ArchiveHeader::new(0, 0, archive_number).to_bytes(),
        )?;
        Ok(Self {
            w: chunk_writer.into_inner(),
            archive_number,
        })
    }

    pub fn add_entry(&mut self, entry: impl Entry) -> io::Result<usize> {
        let bytes = entry.into_bytes();
        self.w.write_all(&bytes)?;
        Ok(bytes.len())
    }

    fn add_next_archive_marker(&mut self) -> io::Result<()> {
        let mut chunk_writer = ChunkWriter::from(&mut self.w);
        chunk_writer.write_chunk(ChunkType::ANXT, &[])
    }

    pub fn split_to_next_archive<OW: Write>(mut self, writer: OW) -> io::Result<ArchiveWriter<OW>> {
        let next_archive_number = self.archive_number + 1;
        self.add_next_archive_marker()?;
        self.finalize()?;
        ArchiveWriter::write_header_with_archive_number(writer, next_archive_number)
    }

    pub fn finalize(mut self) -> io::Result<W> {
        let mut chunk_writer = ChunkWriter::from(&mut self.w);
        chunk_writer.write_chunk(ChunkType::AEND, &[])?;
        Ok(self.w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode() {
        let writer = ArchiveWriter::write_header(Vec::new()).unwrap();
        let file = writer.finalize().unwrap();
        let expected = include_bytes!("../../../resources/test/empty.pna");
        assert_eq!(file.as_slice(), expected.as_slice());
    }
}
