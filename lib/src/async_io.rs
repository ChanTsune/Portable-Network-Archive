//! Asynchronous I/O primitives for reading and writing PNA archives.

use crate::{
    Chunk, ChunkType, MIN_CHUNK_BYTES_SIZE, PNA_SIGNATURE, RawChunk, util::io::try_zeroed_vec,
};
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite};
use futures_util::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use std::{
    io::{self, SeekFrom},
    mem,
};

/// Reads and validates the PNA archive signature asynchronously.
///
/// On success, `reader` has consumed exactly the signature bytes. On failure an
/// unspecified number of bytes has been consumed, so `reader` cannot be reused
/// to probe for another format.
///
/// # Errors
///
/// Returns [`io::ErrorKind::UnexpectedEof`] when the signature cannot be fully
/// read, [`io::ErrorKind::InvalidData`] when it does not match, and any other
/// error produced by `reader`.
#[inline]
pub async fn read_signature<R: AsyncRead + Unpin + ?Sized>(reader: &mut R) -> io::Result<()> {
    let mut signature = [0u8; PNA_SIGNATURE.len()];
    reader.read_exact(&mut signature).await?;
    crate::format::validate_signature(&signature)
}

/// Reads and validates one PNA chunk from `reader` asynchronously.
///
/// The reader must be positioned at the chunk length field. On success, this
/// function consumes exactly one complete chunk and does not read any bytes
/// following its CRC. This function does not read the archive signature or
/// interpret archive-level chunk ordering.
///
/// `max_data_len` is an inclusive upper bound for the chunk data length. Pass
/// [`u32::MAX`] to allow the full range representable by the PNA format.
///
/// On failure, an unspecified number of bytes may have been consumed and the
/// reader is not guaranteed to remain at a chunk boundary.
///
/// # Errors
///
/// Returns [`io::ErrorKind::UnexpectedEof`] when any chunk field is incomplete,
/// [`io::ErrorKind::InvalidData`] when the declared data length exceeds
/// `max_data_len`, the chunk type is invalid, or the stored CRC-32 does not
/// match the chunk type and data, [`io::ErrorKind::OutOfMemory`] when the chunk
/// data buffer cannot be allocated, and any other error produced by `reader`.
#[inline]
pub async fn read_chunk<R: AsyncRead + Unpin + ?Sized>(
    reader: &mut R,
    max_data_len: u32,
) -> io::Result<RawChunk<Vec<u8>>> {
    let mut length = [0u8; mem::size_of::<u32>()];
    reader.read_exact(&mut length).await?;
    let length = u32::from_be_bytes(length);
    crate::format::check_chunk_length_limit(length, max_data_len)?;

    let mut ty = [0u8; mem::size_of::<ChunkType>()];
    reader.read_exact(&mut ty).await?;
    let chunk_type = ChunkType::new(ty)?;

    let mut data = try_zeroed_vec(length as usize)?;
    reader.read_exact(&mut data).await?;

    let mut crc = [0u8; mem::size_of::<u32>()];
    reader.read_exact(&mut crc).await?;
    let crc = u32::from_be_bytes(crc);
    crate::format::validate_chunk_crc(&ty, &data, crc)?;

    Ok(RawChunk {
        length,
        ty: chunk_type,
        data,
        crc,
    })
}

/// Writes one PNA chunk to `writer` asynchronously.
///
/// This function writes the values reported by [`Chunk::length`],
/// [`Chunk::ty`], [`Chunk::data`], and [`Chunk::crc`] without validating or
/// recalculating them. To derive the default length and CRC from a type and
/// data, pass a `(ChunkType, data)` tuple.
///
/// The archive signature and archive-level chunk ordering are not written or
/// interpreted. This function does not flush `writer`.
///
/// On success, the returned value is the number of bytes actually written,
/// based on the size of [`Chunk::data`] rather than [`Chunk::length`]. On
/// failure, a prefix of the chunk may already have been written.
///
/// # Errors
///
/// Returns [`io::ErrorKind::InvalidInput`] if the actual output length cannot
/// be represented by [`usize`], and any error produced by `writer`.
#[inline]
pub async fn write_chunk<W: AsyncWrite + Unpin + ?Sized>(
    writer: &mut W,
    chunk: impl Chunk,
) -> io::Result<usize> {
    let length = chunk.length().to_be_bytes();
    let ty = chunk.ty();
    let data = chunk.data();
    let crc = chunk.crc().to_be_bytes();
    let bytes_len = MIN_CHUNK_BYTES_SIZE
        .checked_add(data.len())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "chunk byte length overflow"))?;

    writer.write_all(&length).await?;
    writer.write_all(ty.as_bytes()).await?;
    writer.write_all(data).await?;
    writer.write_all(&crc).await?;
    Ok(bytes_len)
}

/// Skips one PNA chunk on `reader` asynchronously.
///
/// The reader must be positioned at the chunk length field. On success, this
/// function consumes exactly one complete chunk and does not read any bytes
/// following its CRC. The chunk data is skipped based on the declared data
/// length, and neither the data nor the CRC is validated. This function does
/// not read the archive signature or interpret archive-level chunk ordering.
///
/// Returns the chunk type and the number of bytes consumed, which is
/// [`MIN_CHUNK_BYTES_SIZE`] plus the declared data length.
///
/// On failure, an unspecified number of bytes may have been consumed and the
/// reader is not guaranteed to remain at a chunk boundary.
///
/// # Errors
///
/// Returns [`io::ErrorKind::UnexpectedEof`] when the length or type field
/// cannot be fully read, or when the CRC field cannot be read after skipping
/// the data, [`io::ErrorKind::InvalidData`] when the chunk type is invalid,
/// and any other error produced by `reader`.
#[inline]
pub async fn skip_chunk<R: AsyncRead + AsyncSeek + Unpin + ?Sized>(
    reader: &mut R,
) -> io::Result<(ChunkType, u64)> {
    let mut length = [0u8; mem::size_of::<u32>()];
    reader.read_exact(&mut length).await?;
    let length = u32::from_be_bytes(length);

    let mut ty = [0u8; mem::size_of::<ChunkType>()];
    reader.read_exact(&mut ty).await?;
    let ty = ChunkType::new(ty)?;

    reader.seek(SeekFrom::Current(length.into())).await?;

    let mut crc = [0u8; mem::size_of::<u32>()];
    reader.read_exact(&mut crc).await?;

    Ok((ty, MIN_CHUNK_BYTES_SIZE as u64 + u64::from(length)))
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use futures_util::io::Cursor;
    use std::pin::Pin;
    use std::task::{Context, Poll, ready};

    #[tokio::test]
    async fn read_signature_consumes_exactly_the_signature() {
        let input = [PNA_SIGNATURE.as_slice(), b"body"].concat();
        let mut reader = Cursor::new(input);
        read_signature(&mut reader).await.unwrap();
        assert_eq!(reader.position(), PNA_SIGNATURE.len() as u64);
    }

    #[tokio::test]
    async fn read_signature_rejects_input_one_byte_short() {
        let mut reader = Cursor::new(&PNA_SIGNATURE[..7]);
        assert_eq!(
            read_signature(&mut reader).await.unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
    }

    #[tokio::test]
    async fn read_signature_rejects_mismatched_signature() {
        let mut reader = Cursor::new(b"xxxxxxxx");
        assert_eq!(
            read_signature(&mut reader).await.unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    fn valid_chunk_bytes() -> Vec<u8> {
        vec![
            0x00, 0x00, 0x00, 0x04, // chunk length
            b'F', b'D', b'A', b'T', // chunk type
            0xAA, 0xBB, 0xCC, 0xDD, // chunk data
            0x47, 0xF3, 0x2B, 0x10, // CRC-32
        ]
    }

    fn raw_chunk_bytes(ty: [u8; 4], data: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(data.len() + 12);
        bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&ty);
        bytes.extend_from_slice(data);
        bytes.extend_from_slice(&crate::format::chunk_crc(&ty, data).to_be_bytes());
        bytes
    }

    #[tokio::test]
    async fn read_chunk_returns_owned_chunk_and_stops_at_its_crc() {
        let mut input = valid_chunk_bytes();
        let chunk_len = input.len();
        input.extend_from_slice(b"following");
        let mut reader = Cursor::new(input);

        let chunk = read_chunk(&mut reader, u32::MAX).await.unwrap();

        assert_eq!(chunk.ty(), ChunkType::FDAT);
        assert_eq!(chunk.data(), [0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(reader.position(), chunk_len as u64);
    }

    #[tokio::test]
    async fn read_chunk_enforces_inclusive_data_length_limit() {
        let mut reader = Cursor::new(raw_chunk_bytes(*b"AEND", &[]));
        let chunk = read_chunk(&mut reader, 0).await.unwrap();
        assert_eq!(chunk.ty(), ChunkType::AEND);
        assert!(chunk.data().is_empty());

        let mut reader = Cursor::new(valid_chunk_bytes());
        assert_eq!(read_chunk(&mut reader, 4).await.unwrap().length(), 4);

        let mut reader = Cursor::new(valid_chunk_bytes());
        assert_eq!(
            read_chunk(&mut reader, 3).await.unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[tokio::test]
    async fn read_chunk_checks_limit_before_requiring_the_remaining_fields() {
        let mut reader = Cursor::new(u32::MAX.to_be_bytes());
        assert_eq!(
            read_chunk(&mut reader, 1024).await.unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[tokio::test]
    async fn read_chunk_rejects_every_truncation_boundary() {
        let bytes = valid_chunk_bytes();
        for end in 0..bytes.len() {
            let mut reader = Cursor::new(&bytes[..end]);
            assert_eq!(
                read_chunk(&mut reader, u32::MAX).await.unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "truncation at byte {end}",
            );
        }
    }

    #[tokio::test]
    async fn read_chunk_applies_chunk_type_validation_rules() {
        let mut reader = Cursor::new(raw_chunk_bytes(*b"FD1T", b"data"));
        assert_eq!(
            read_chunk(&mut reader, u32::MAX).await.unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        let mut reader = Cursor::new(raw_chunk_bytes(*b"ABcD", b"data"));
        assert_eq!(
            read_chunk(&mut reader, u32::MAX).await.unwrap().data(),
            b"data"
        );
    }

    #[tokio::test]
    async fn read_chunk_rejects_crc_mismatch() {
        let mut bytes = valid_chunk_bytes();
        *bytes.last_mut().unwrap() ^= 0xFF;
        let mut reader = Cursor::new(bytes);
        assert_eq!(
            read_chunk(&mut reader, u32::MAX).await.unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[tokio::test]
    async fn write_chunk_preserves_reported_fields() {
        struct StoredChunk;

        impl Chunk for StoredChunk {
            fn length(&self) -> u32 {
                1
            }

            fn ty(&self) -> ChunkType {
                ChunkType::FDAT
            }

            fn data(&self) -> &[u8] {
                b"abc"
            }

            fn crc(&self) -> u32 {
                0x0102_0304
            }
        }

        let mut output = Cursor::new(Vec::new());
        let written = write_chunk(&mut output, StoredChunk).await.unwrap();

        assert_eq!(written, 15);
        assert_eq!(
            output.into_inner(),
            [
                0, 0, 0, 1, b'F', b'D', b'A', b'T', b'a', b'b', b'c', 1, 2, 3, 4
            ]
        );
    }

    struct CountingReader<R> {
        inner: R,
        read_bytes: usize,
    }

    impl<R: AsyncRead + Unpin> AsyncRead for CountingReader<R> {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            let this = self.get_mut();
            let n = ready!(Pin::new(&mut this.inner).poll_read(cx, buf))?;
            this.read_bytes += n;
            Poll::Ready(Ok(n))
        }
    }

    impl<R: AsyncSeek + Unpin> AsyncSeek for CountingReader<R> {
        fn poll_seek(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            pos: io::SeekFrom,
        ) -> Poll<io::Result<u64>> {
            Pin::new(&mut self.get_mut().inner).poll_seek(cx, pos)
        }
    }

    /// Supplies fixed-size fields without holding a data buffer and records
    /// the requested seek distance.
    struct RecordingSeekReader {
        header: Vec<u8>,
        pos: usize,
        trailer_left: usize,
        seek_from: Option<io::SeekFrom>,
    }

    impl AsyncRead for RecordingSeekReader {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<io::Result<usize>> {
            let this = self.get_mut();
            if this.pos < this.header.len() {
                let n = buf.len().min(this.header.len() - this.pos);
                buf[..n].copy_from_slice(&this.header[this.pos..this.pos + n]);
                this.pos += n;
                Poll::Ready(Ok(n))
            } else if this.trailer_left > 0 {
                let n = buf.len().min(this.trailer_left);
                buf[..n].fill(0);
                this.trailer_left -= n;
                Poll::Ready(Ok(n))
            } else {
                Poll::Ready(Ok(0))
            }
        }
    }

    impl AsyncSeek for RecordingSeekReader {
        fn poll_seek(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            pos: io::SeekFrom,
        ) -> Poll<io::Result<u64>> {
            self.get_mut().seek_from = Some(pos);
            Poll::Ready(Ok(0))
        }
    }

    #[tokio::test]
    async fn skip_chunk_returns_type_and_size_without_reading_data() {
        let mut input = valid_chunk_bytes();
        let chunk_len = input.len();
        input.extend_from_slice(b"following");
        let mut reader = CountingReader {
            inner: Cursor::new(input),
            read_bytes: 0,
        };

        let (ty, consumed) = skip_chunk(&mut reader).await.unwrap();

        assert_eq!(ty, ChunkType::FDAT);
        assert_eq!(consumed, chunk_len as u64);
        assert_eq!(reader.inner.position(), consumed);
        assert_eq!(reader.read_bytes, MIN_CHUNK_BYTES_SIZE);
    }

    #[tokio::test]
    async fn skip_chunk_seeks_over_the_maximum_data_length() {
        let mut reader = RecordingSeekReader {
            header: [&u32::MAX.to_be_bytes()[..], b"FDAT"].concat(),
            pos: 0,
            trailer_left: 4,
            seek_from: None,
        };
        let (ty, consumed) = skip_chunk(&mut reader).await.unwrap();
        assert_eq!(ty, ChunkType::FDAT);
        assert_eq!(consumed, u64::from(u32::MAX) + MIN_CHUNK_BYTES_SIZE as u64);
        assert_eq!(
            reader.seek_from,
            Some(io::SeekFrom::Current(i64::from(u32::MAX)))
        );
    }

    #[tokio::test]
    async fn skip_chunk_rejects_invalid_chunk_type_before_seeking() {
        let mut reader = Cursor::new([&4u32.to_be_bytes()[..], b"FD1T"].concat());
        assert_eq!(
            skip_chunk(&mut reader).await.unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(reader.position(), 8);
    }

    #[tokio::test]
    async fn skip_chunk_rejects_every_truncation_boundary() {
        let bytes = valid_chunk_bytes();
        for end in 0..bytes.len() {
            let mut reader = Cursor::new(&bytes[..end]);
            assert_eq!(
                skip_chunk(&mut reader).await.unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "truncation at byte {end}",
            );
        }
    }

    #[tokio::test]
    async fn skip_chunk_handles_empty_data() {
        let mut reader = Cursor::new(raw_chunk_bytes(*b"AEND", &[]));
        let (ty, consumed) = skip_chunk(&mut reader).await.unwrap();
        assert_eq!(ty, ChunkType::AEND);
        assert_eq!(consumed, MIN_CHUNK_BYTES_SIZE as u64);
    }

    #[tokio::test]
    async fn skip_chunk_does_not_validate_crc() {
        let mut bytes = valid_chunk_bytes();
        *bytes.last_mut().unwrap() ^= 0xFF;
        let mut reader = Cursor::new(bytes);
        assert_eq!(skip_chunk(&mut reader).await.unwrap().0, ChunkType::FDAT);
    }
}
