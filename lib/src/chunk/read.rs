//! Chunk reading and deserialization from byte streams and slices.

use crate::chunk::{ChunkType, MIN_CHUNK_BYTES_SIZE, RawChunk};
use core::num::NonZeroU32;
use std::{
    io::{self, Read, Seek, SeekFrom},
    mem,
};

pub(crate) struct ChunkReader<R> {
    pub(crate) r: R,
    max_chunk_size: Option<NonZeroU32>,
}

impl<R> ChunkReader<R> {
    pub(crate) fn new(reader: R, max_chunk_size: Option<NonZeroU32>) -> Self {
        Self {
            r: reader,
            max_chunk_size,
        }
    }
}

impl<R: Read> ChunkReader<R> {
    #[inline]
    pub(crate) fn read_chunk(&mut self) -> io::Result<RawChunk> {
        read_chunk(&mut self.r, self.max_chunk_size)
    }
}

impl<R: Read + Seek> ChunkReader<R> {
    pub(crate) fn skip_chunk(&mut self) -> io::Result<(ChunkType, usize)> {
        // read chunk length
        let mut length = [0u8; mem::size_of::<u32>()];
        self.r.read_exact(&mut length)?;
        let length = u32::from_be_bytes(length);

        // read a chunk type
        let mut ty = [0u8; mem::size_of::<ChunkType>()];
        self.r.read_exact(&mut ty)?;

        // skip chunk data
        self.r.seek(SeekFrom::Current(length.into()))?;

        // skip crc sum
        self.r
            .seek(SeekFrom::Current(mem::size_of::<u32>() as i64))?;

        Ok((
            ChunkType::new(ty)?,
            MIN_CHUNK_BYTES_SIZE
                .checked_add(length as usize)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "chunk size overflow"))?,
        ))
    }
}

pub(crate) fn read_chunk<R: Read>(
    mut r: R,
    max_chunk_size: Option<NonZeroU32>,
) -> io::Result<RawChunk> {
    crate::io::read_chunk(&mut r, max_chunk_size.map_or(u32::MAX, NonZeroU32::get))
}

pub(crate) fn read_chunk_from_slice(bytes: &[u8]) -> io::Result<(RawChunk<&[u8]>, &[u8])> {
    crate::bytes::read_chunk(bytes, u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::ChunkExt;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    fn valid_chunk_bytes() -> Vec<u8> {
        RawChunk::from_data(ChunkType::FDAT, vec![0xAA, 0xBB, 0xCC, 0xDD]).to_bytes()
    }

    #[test]
    fn read_from_slice_roundtrips_valid_chunk() {
        let bytes = valid_chunk_bytes();
        let (chunk, rest) = read_chunk_from_slice(&bytes).unwrap();
        assert_eq!(chunk.ty, ChunkType::FDAT);
        assert_eq!(chunk.data, &[0xAA, 0xBB, 0xCC, 0xDD]);
        assert!(rest.is_empty());
    }

    #[test]
    fn read_from_slice_rejects_crc_mismatch() {
        let mut bytes = valid_chunk_bytes();
        *bytes.last_mut().unwrap() ^= 0xFF;
        let err = read_chunk_from_slice(&bytes).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_from_slice_rejects_data_corruption_with_intact_crc() {
        let mut bytes = valid_chunk_bytes();
        bytes[8] ^= 0xFF;
        let err = read_chunk_from_slice(&bytes).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_from_slice_rejects_truncated_crc() {
        let bytes = valid_chunk_bytes();
        let truncated = &bytes[..bytes.len() - 2];
        let err = read_chunk_from_slice(truncated).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn read_from_slice_rejects_length_exceeding_input() {
        let mut bytes = valid_chunk_bytes();
        bytes[3] = 0xFF;
        let err = read_chunk_from_slice(&bytes).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn read_from_reader_roundtrips_valid_chunk() {
        let bytes = valid_chunk_bytes();
        let chunk = read_chunk(io::Cursor::new(bytes), None).unwrap();
        assert_eq!(chunk.ty, ChunkType::FDAT);
        assert_eq!(chunk.data, vec![0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn read_from_reader_rejects_crc_mismatch() {
        let mut bytes = valid_chunk_bytes();
        *bytes.last_mut().unwrap() ^= 0xFF;
        let err = read_chunk(io::Cursor::new(bytes), None).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn read_from_reader_rejects_length_exceeding_input() {
        let mut bytes = valid_chunk_bytes();
        bytes[3] = 0xFF;
        let err = read_chunk(io::Cursor::new(bytes), None).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::UnexpectedEof);
    }
}
