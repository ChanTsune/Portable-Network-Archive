use crate::{
    archive::PNA_HEADER,
    chunk::{ChunkType, ChunkWriter},
    create_chunk_data_ahed, Entry,
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
    finalized: bool,
}

impl<W: Write> ArchiveWriter<W> {
    fn write_header(mut write: W) -> io::Result<Self> {
        write.write_all(PNA_HEADER)?;
        let mut chunk_writer = ChunkWriter::from(write);
        chunk_writer.write_chunk(ChunkType::AHED, &create_chunk_data_ahed(0, 0, 0))?;
        Ok(Self {
            w: chunk_writer.into_inner(),
            finalized: false,
        })
    }

    pub fn add_entry(&mut self, entry: impl Entry) -> io::Result<usize> {
        let bytes = entry.into_bytes();
        self.w.write_all(&bytes)?;
        Ok(bytes.len())
    }

    pub fn finalize(&mut self) -> io::Result<()> {
        if !self.finalized {
            let mut chunk_writer = ChunkWriter::from(&mut self.w);
            chunk_writer.write_chunk(ChunkType::AEND, &[])?;
            self.finalized = true;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Encoder;

    #[test]
    fn encode() {
        let mut file = Vec::new();
        {
            let encoder = Encoder::new();
            let mut writer = encoder.write_header(&mut file).unwrap();
            writer.finalize().unwrap();
        }
        let expected = include_bytes!("../../../resources/test/empty.pna");
        assert_eq!(file.as_slice(), expected.as_slice());
    }
}
