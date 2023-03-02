use crate::chunk::{crc::Crc32, ChunkType};
use std::io::{self, Write};

pub struct ChunkWriter<W> {
    w: W,
}

impl<W> From<W> for ChunkWriter<W>
where
    W: Write,
{
    fn from(writer: W) -> Self {
        Self { w: writer }
    }
}

impl<W: Write> ChunkWriter<W> {
    pub fn write_chunk(&mut self, type_: ChunkType, data: &[u8]) -> io::Result<usize> {
        let mut crc = Crc32::new();
        // write length
        let length = data.len() as u32;
        self.w.write_all(&length.to_be_bytes())?;

        // write chunk type
        self.w.write_all(&type_.0)?;
        crc.update(&type_.0);

        // write data
        self.w.write_all(data)?;
        crc.update(data);

        // write crc32
        self.w.write_all(&crc.finalize().to_be_bytes())?;

        // NOTE: chunk_type.len() + length.len() + data.len() + crc.len()
        Ok(4 + 4 + data.len() + 4)
    }
}
