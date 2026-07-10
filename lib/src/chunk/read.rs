//! Chunk reading and deserialization from byte streams and slices.

use crate::chunk::{ChunkType, MIN_CHUNK_BYTES_SIZE, RawChunk, crc::Crc32};
use core::num::NonZeroU32;
#[cfg(feature = "unstable-async")]
use futures_io::AsyncRead;
#[cfg(feature = "unstable-async")]
use futures_util::AsyncReadExt;
use std::{
    io::{self, Read, Seek, SeekFrom},
    mem,
};

/// Allocate chunk data buffer with graceful OOM handling and optional size limit.
fn allocate_chunk_data(length: u32, max_size: Option<NonZeroU32>) -> io::Result<Vec<u8>> {
    if let Some(max) = max_size
        && length > max.get()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("chunk size {} exceeds limit {}", length, max),
        ));
    }

    let len = length as usize;
    let mut data = Vec::new();
    data.try_reserve_exact(len).map_err(|_| {
        io::Error::new(
            io::ErrorKind::OutOfMemory,
            format!("failed to allocate {} bytes for chunk", len),
        )
    })?;
    data.resize(len, 0);
    Ok(data)
}

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

#[cfg(feature = "unstable-async")]
impl<R: AsyncRead + Unpin> ChunkReader<R> {
    pub(crate) async fn read_chunk_async(&mut self) -> io::Result<RawChunk> {
        let mut crc_hasher = Crc32::new();

        // read chunk length
        let mut length = [0u8; mem::size_of::<u32>()];
        self.r.read_exact(&mut length).await?;
        let length = u32::from_be_bytes(length);

        // read a chunk type
        let mut ty = [0u8; mem::size_of::<ChunkType>()];
        self.r.read_exact(&mut ty).await?;

        crc_hasher.update(&ty);

        // read chunk data
        let mut data = allocate_chunk_data(length, self.max_chunk_size)?;
        self.r.read_exact(&mut data).await?;

        crc_hasher.update(&data);

        // read crc sum
        let mut crc = [0u8; mem::size_of::<u32>()];
        self.r.read_exact(&mut crc).await?;
        let crc = u32::from_be_bytes(crc);

        if crc != crc_hasher.finalize() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "broken chunk"));
        }
        let ty = ChunkType::new(ty)?;
        Ok(RawChunk {
            length,
            ty,
            data,
            crc,
        })
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
    let mut crc_hasher = Crc32::new();

    // read chunk length
    let mut length = [0u8; mem::size_of::<u32>()];
    r.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length);

    // read a chunk type
    let mut ty = [0u8; mem::size_of::<ChunkType>()];
    r.read_exact(&mut ty)?;

    crc_hasher.update(&ty);

    // read chunk data
    let mut data = allocate_chunk_data(length, max_chunk_size)?;
    r.read_exact(&mut data)?;

    crc_hasher.update(&data);

    // read crc sum
    let mut crc = [0u8; mem::size_of::<u32>()];
    r.read_exact(&mut crc)?;
    let crc = u32::from_be_bytes(crc);

    if crc != crc_hasher.finalize() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "broken chunk"));
    }
    let ty = ChunkType::new(ty)?;
    Ok(RawChunk {
        length,
        ty,
        data,
        crc,
    })
}

pub(crate) fn read_chunk_from_slice(bytes: &[u8]) -> io::Result<(RawChunk<&[u8]>, &[u8])> {
    let mut crc_hasher = Crc32::new();

    // read chunk length
    let (length, r) = bytes
        .split_first_chunk::<{ mem::size_of::<u32>() }>()
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    let length = u32::from_be_bytes(*length);

    // read a chunk type
    let (ty, r) = r
        .split_first_chunk::<{ mem::size_of::<ChunkType>() }>()
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    crc_hasher.update(&ty[..]);

    // read chunk data
    let (data, r) = r
        .split_at_checked(length as usize)
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    crc_hasher.update(data);

    // read crc sum
    let (crc, r) = r
        .split_first_chunk::<{ mem::size_of::<u32>() }>()
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    let crc = u32::from_be_bytes(*crc);

    if crc != crc_hasher.finalize() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "broken chunk"));
    }
    let ty = ChunkType::new(*ty)?;
    Ok((
        RawChunk {
            length,
            ty,
            data,
            crc,
        },
        r,
    ))
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
