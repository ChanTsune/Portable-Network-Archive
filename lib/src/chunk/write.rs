//! Chunk writing and serialization to byte streams.

use crate::chunk::{Chunk, ChunkExt, ChunkType};
use core::num::NonZeroU32;
#[cfg(feature = "unstable-async")]
use futures_io::AsyncWrite;
#[cfg(feature = "unstable-async")]
use futures_util::AsyncWriteExt;
use std::io::{self, Write};

/// Writes the entire chunk to a given writer in a single pass.
///
/// This function calculates the CRC32 checksum on-the-fly as it writes the
/// chunk type and data, reducing the number of passes over the data from
/// two to one. This is especially beneficial for large file data chunks.
pub(crate) fn write_chunk_single_pass_in<W: Write>(
    chunk: impl Chunk,
    writer: &mut W,
) -> io::Result<usize> {
    writer.write_all(&chunk.length().to_be_bytes())?;
    let crc = {
        let mut crc_writer = CrcWriter::new(&mut *writer);
        crc_writer.write_all(chunk.ty().as_bytes())?;
        crc_writer.write_all(chunk.data())?;
        crc_writer.crc.finalize()
    };
    writer.write_all(&crc.to_be_bytes())?;
    Ok(chunk.bytes_len())
}

struct CrcWriter<W> {
    w: W,
    crc: crate::chunk::Crc32,
}

impl<W: Write> CrcWriter<W> {
    fn new(w: W) -> Self {
        Self {
            w,
            crc: crate::chunk::Crc32::new(),
        }
    }
}

impl<W: Write> Write for CrcWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.w.write(buf)?;
        self.crc.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

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

impl<W: Write> Write for ChunkStreamWriter<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let chunk = &buf[..buf.len().min(self.max_chunk_size)];
        write_chunk_single_pass_in((self.ty, chunk), &mut self.w)?;
        Ok(chunk.len())
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;


    #[test]
    fn write_aend_chunk() {
        let mut buf = Vec::new();
        assert_eq!(
            write_chunk_single_pass_in((ChunkType::AEND, []), &mut buf).unwrap(),
            12
        );
        assert_eq!(buf, [0, 0, 0, 0, 65, 69, 78, 68, 107, 246, 72, 109]);
    }

    #[test]
    fn write_fdat_chunk() {
        let mut buf = Vec::new();
        assert_eq!(
            write_chunk_single_pass_in((ChunkType::FDAT, b"text data"), &mut buf).unwrap(),
            21,
        );

        assert_eq!(
            buf,
            [
                0, 0, 0, 9, 70, 68, 65, 84, 116, 101, 120, 116, 32, 100, 97, 116, 97, 177, 70, 138,
                128
            ]
        );
    }

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

    #[test]
    fn stream_writer_exact_max_produces_single_chunk() {
        let mut writer = ChunkStreamWriter::new(ChunkType::FDAT, Vec::new(), NonZeroU32::new(4));
        let n = writer.write(b"abcd").unwrap();
        assert_eq!(n, 4);
        let out = writer.into_inner();
        assert_eq!(&out[0..4], &4u32.to_be_bytes());
        assert_eq!(&out[8..12], b"abcd");
        assert_eq!(out.len(), 16);
    }

    #[test]
    fn stream_writer_one_over_max_produces_two_chunks() {
        let mut writer = ChunkStreamWriter::new(ChunkType::FDAT, Vec::new(), NonZeroU32::new(4));
        writer.write_all(b"abcde").unwrap();
        let out = writer.into_inner();
        assert_eq!(&out[0..4], &4u32.to_be_bytes());
        assert_eq!(&out[8..12], b"abcd");
        assert_eq!(&out[16..20], &1u32.to_be_bytes());
        assert_eq!(&out[24..25], b"e");
        assert_eq!(out.len(), 16 + 13);
    }
}
