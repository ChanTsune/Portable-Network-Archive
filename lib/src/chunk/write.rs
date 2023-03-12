use crate::chunk::{crc::Crc32, ChunkType};
use std::io::{self, Write};

pub struct ChunkWriter<W> {
    w: W,
}

impl<W> ChunkWriter<W> {
    pub(crate) fn into_inner(self) -> W {
        self.w
    }
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
    pub fn write_chunk(&mut self, type_: ChunkType, data: &[u8]) -> io::Result<()> {
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::*;
    use super::*;

    #[test]
    fn write_aend_chunk() {
        let mut chunk_writer = ChunkWriter::from(Vec::new());
        chunk_writer.write_chunk(AEND, &[]).unwrap();
        assert_eq!(
            chunk_writer.into_inner(),
            [0, 0, 0, 0, 65, 69, 78, 68, 107, 246, 72, 109]
        )
    }

    #[test]
    fn write_fdat_chunk() {
        let mut chunk_writer = ChunkWriter::from(Vec::new());
        chunk_writer
            .write_chunk(FDAT, "text data".as_bytes())
            .unwrap();
        assert_eq!(
            chunk_writer.into_inner(),
            [
                0, 0, 0, 9, 70, 68, 65, 84, 116, 101, 120, 116, 32, 100, 97, 116, 97, 177, 70, 138,
                128
            ]
        )
    }
}
