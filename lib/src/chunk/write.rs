use crate::chunk::{Chunk, ChunkExt};
#[cfg(feature = "unstable-async")]
use futures::{AsyncWrite, AsyncWriteExt};
use std::io::{self, Write};

pub(crate) struct ChunkWriter<W> {
    w: W,
}

impl<W> From<W> for ChunkWriter<W> {
    fn from(writer: W) -> Self {
        Self { w: writer }
    }
}

impl<W: Write> ChunkWriter<W> {
    pub(crate) fn write_chunk(&mut self, chunk: impl Chunk) -> io::Result<usize> {
        // write length
        let length = chunk.length();
        self.w.write_all(&length.to_be_bytes())?;

        // write chunk type
        self.w.write_all(&chunk.ty().0)?;

        // write data
        self.w.write_all(chunk.data())?;

        // write crc32
        self.w.write_all(&chunk.crc().to_be_bytes())?;
        Ok(chunk.bytes_len())
    }
}

#[cfg(feature = "unstable-async")]
impl<W: AsyncWrite + Unpin> ChunkWriter<W> {
    pub(crate) async fn write_chunk_async(&mut self, chunk: impl Chunk) -> io::Result<usize> {
        // write length
        let length = chunk.length();
        self.w.write_all(&length.to_be_bytes()).await?;

        // write chunk type
        self.w.write_all(&chunk.ty().0).await?;

        // write data
        self.w.write_all(chunk.data()).await?;

        // write crc32
        self.w.write_all(&chunk.crc().to_be_bytes()).await?;
        Ok(chunk.bytes_len())
    }
}

#[cfg(test)]
mod tests {
    use super::super::ChunkType;
    use super::*;

    #[test]
    fn write_aend_chunk() {
        let mut chunk_writer = ChunkWriter::from(Vec::new());
        assert_eq!(
            chunk_writer
                .write_chunk((ChunkType::AEND, [].as_slice()))
                .unwrap(),
            12
        );
        assert_eq!(
            chunk_writer.w,
            [0, 0, 0, 0, 65, 69, 78, 68, 107, 246, 72, 109]
        );
    }

    #[test]
    fn write_fdat_chunk() {
        let mut chunk_writer = ChunkWriter::from(Vec::new());
        assert_eq!(
            chunk_writer
                .write_chunk((ChunkType::FDAT, "text data".as_bytes()))
                .unwrap(),
            21,
        );

        assert_eq!(
            chunk_writer.w,
            [
                0, 0, 0, 9, 70, 68, 65, 84, 116, 101, 120, 116, 32, 100, 97, 116, 97, 177, 70, 138,
                128
            ]
        );
    }
}
