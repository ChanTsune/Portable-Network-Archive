//! Chunk writing and serialization to byte streams.

use crate::{chunk::ChunkType, io::WriteChunk};
use core::num::NonZeroU32;
use std::io::{self, Write};

pub(crate) struct ChunkStreamWriter<W> {
    ty: ChunkType,
    w: W,
    max_chunk_size: usize,
}

impl<W> ChunkStreamWriter<W> {
    #[inline]
    pub(crate) const fn new(ty: ChunkType, inner: W, max_chunk_size: Option<NonZeroU32>) -> Self {
        Self {
            ty,
            w: inner,
            max_chunk_size: match max_chunk_size {
                Some(n) => n.get() as usize,
                None => u32::MAX as usize,
            },
        }
    }

    #[inline]
    pub(crate) fn into_inner(self) -> W {
        self.w
    }
}

impl<W: WriteChunk> Write for ChunkStreamWriter<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let chunk = &buf[..buf.len().min(self.max_chunk_size)];
        self.w.write_chunk((self.ty, chunk))?;
        Ok(chunk.len())
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.w.flush_chunks()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn stream_writer_no_limit_writes_single_chunk() {
        let mut writer = ChunkStreamWriter::new(ChunkType::FDAT, Vec::new(), None);
        let n = writer.write(b"hello world").unwrap();
        assert_eq!(n, 11);
        let out = writer.into_inner();
        assert_eq!(out.len(), 23);
        assert_eq!(&out[0..4], &11u32.to_be_bytes());
    }

    #[test]
    fn stream_writer_write_returns_at_most_max_chunk_size() {
        let mut writer = ChunkStreamWriter::new(ChunkType::FDAT, Vec::new(), NonZeroU32::new(4));
        let n = writer.write(b"abcdefghij").unwrap();
        assert_eq!(n, 4);
        let out = writer.into_inner();
        assert_eq!(&out[0..4], &4u32.to_be_bytes());
        assert_eq!(&out[8..12], b"abcd");
        assert_eq!(out.len(), 16);
    }

    #[test]
    fn stream_writer_write_all_splits_into_multiple_chunks() {
        let mut writer = ChunkStreamWriter::new(ChunkType::FDAT, Vec::new(), NonZeroU32::new(4));
        writer.write_all(b"abcdefghij").unwrap();
        let out = writer.into_inner();

        assert_eq!(&out[0..4], &4u32.to_be_bytes());
        assert_eq!(&out[8..12], b"abcd");

        assert_eq!(&out[16..20], &4u32.to_be_bytes());
        assert_eq!(&out[24..28], b"efgh");

        assert_eq!(&out[32..36], &2u32.to_be_bytes());
        assert_eq!(&out[40..42], b"ij");

        assert_eq!(out.len(), 16 + 16 + 14);
    }

    #[test]
    fn stream_writer_empty_write_produces_no_output() {
        let mut writer = ChunkStreamWriter::new(ChunkType::FDAT, Vec::new(), NonZeroU32::new(4));
        let n = writer.write(b"").unwrap();
        assert_eq!(n, 0);
        let out = writer.into_inner();
        assert_eq!(out.len(), 0);
    }
}
