use crate::chunk::{Chunk, ChunkExt, ChunkType};
#[cfg(feature = "unstable-async")]
use futures_io::AsyncWrite;
#[cfg(feature = "unstable-async")]
use futures_util::AsyncWriteExt;
use std::io::{self, Write};

pub(crate) struct ChunkWriter<W> {
    w: W,
}

impl<W> ChunkWriter<W> {
    #[inline]
    pub(crate) const fn new(writer: W) -> Self {
        Self { w: writer }
    }
}

impl<W: Write> ChunkWriter<W> {
    #[inline]
    pub(crate) fn write_chunk(&mut self, chunk: impl Chunk) -> io::Result<usize> {
        chunk.write_chunk_in(&mut self.w)
    }
}

#[cfg(feature = "unstable-async")]
impl<W: AsyncWrite + Unpin> ChunkWriter<W> {
    pub(crate) async fn write_chunk_async(&mut self, chunk: impl Chunk) -> io::Result<usize> {
        // write length
        let length = chunk.length();
        self.w.write_all(&length.to_be_bytes()).await?;

        // write a chunk type
        self.w.write_all(&chunk.ty().0).await?;

        // write data
        self.w.write_all(chunk.data()).await?;

        // write crc32
        self.w.write_all(&chunk.crc().to_be_bytes()).await?;
        Ok(chunk.bytes_len())
    }
}

pub(crate) struct ChunkStreamWriter<W> {
    ty: ChunkType,
    w: ChunkWriter<W>,
}

impl<W> ChunkStreamWriter<W> {
    #[inline]
    pub(crate) const fn new(ty: ChunkType, inner: W) -> Self {
        Self {
            ty,
            w: ChunkWriter::new(inner),
        }
    }

    #[inline]
    pub(crate) fn into_inner(self) -> W {
        self.w.w
    }
}

impl<W: Write> Write for ChunkStreamWriter<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.w.write_chunk((self.ty, buf))?;
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.w.w.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn write_aend_chunk() {
        let mut chunk_writer = ChunkWriter::new(Vec::new());
        assert_eq!(chunk_writer.write_chunk((ChunkType::AEND, [])).unwrap(), 12);
        assert_eq!(
            chunk_writer.w,
            [0, 0, 0, 0, 65, 69, 78, 68, 107, 246, 72, 109]
        );
    }

    #[test]
    fn write_fdat_chunk() {
        let mut chunk_writer = ChunkWriter::new(Vec::new());
        assert_eq!(
            chunk_writer
                .write_chunk((ChunkType::FDAT, b"text data"))
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
