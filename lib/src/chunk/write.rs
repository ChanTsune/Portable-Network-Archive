use crate::chunk::{Chunk, ChunkType};
use std::{
    io::{self, Write},
    mem,
};

pub(crate) struct ChunkWriter<W> {
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
    pub(crate) fn write_chunk(&mut self, type_: ChunkType, data: &[u8]) -> io::Result<usize> {
        let chunk = (type_, data);

        // write length
        let length = chunk.length();
        self.w.write_all(&length.to_be_bytes())?;

        // write chunk type
        self.w.write_all(&chunk.ty().0)?;

        // write data
        self.w.write_all(chunk.data())?;

        // write crc32
        self.w.write_all(&chunk.crc().to_be_bytes())?;
        Ok(mem::align_of::<u32>() + chunk.data().len() + chunk.ty().len() + mem::align_of::<u32>())
    }
}

#[cfg(test)]
mod tests {
    use super::super::ChunkType;
    use super::*;

    #[test]
    fn write_aend_chunk() {
        let mut chunk_writer = ChunkWriter::from(Vec::new());
        assert_eq!(chunk_writer.write_chunk(ChunkType::AEND, &[]).unwrap(), 12);
        assert_eq!(
            chunk_writer.into_inner(),
            [0, 0, 0, 0, 65, 69, 78, 68, 107, 246, 72, 109]
        );
    }

    #[test]
    fn write_fdat_chunk() {
        let mut chunk_writer = ChunkWriter::from(Vec::new());
        assert_eq!(
            chunk_writer
                .write_chunk(ChunkType::FDAT, "text data".as_bytes())
                .unwrap(),
            21,
        );

        assert_eq!(
            chunk_writer.into_inner(),
            [
                0, 0, 0, 9, 70, 68, 65, 84, 116, 101, 120, 116, 32, 100, 97, 116, 97, 177, 70, 138,
                128
            ]
        );
    }
}
