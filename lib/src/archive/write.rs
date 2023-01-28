use crate::{
    archive::PNA_HEADER,
    chunk::{self, ChunkWriter},
    create_chunk_data_ahed,
};
use std::io::{self, Write};

#[derive(Default)]
pub struct Encoder;

impl Encoder {
    pub fn new() -> Self {
        Self
    }

    pub fn write_header<W: Write>(&self, mut write: W) -> io::Result<ArchiveWriter<W>> {
        write.write_all(PNA_HEADER)?;
        let mut chunk_writer = ChunkWriter::from(write);
        chunk_writer.write_chunk(chunk::AHED, &create_chunk_data_ahed(0, 0, 0))?;
        Ok(ArchiveWriter {
            w: chunk_writer,
            finalized: false,
        })
    }
}

pub struct ArchiveWriter<W: Write> {
    w: ChunkWriter<W>,
    finalized: bool,
}

impl<W: Write> ArchiveWriter<W> {
    pub fn finalize(&mut self) -> io::Result<()> {
        if !self.finalized {
            self.w.write_chunk(chunk::AEND, &[])?;
            self.finalized = true;
        }
        Ok(())
    }
}

impl<W: Write> Drop for ArchiveWriter<W> {
    fn drop(&mut self) {
        self.finalize().expect("archive finalize failed.");
    }
}

#[cfg(test)]
mod tests {
    use super::Encoder;

    #[test]
    fn encode() {
        let file = tempfile::tempfile().unwrap();
        let encoder = Encoder::new();
        let mut writer = encoder.write_header(file).unwrap();
        writer.finalize().unwrap()
    }
}
