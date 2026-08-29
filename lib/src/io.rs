//! I/O primitives for reading and writing PNA archives.

use crate::{
    Chunk, ChunkType, MIN_CHUNK_BYTES_SIZE, PNA_SIGNATURE, RawChunk, util::io::try_zeroed_vec,
};
use std::{io, mem};

pub(crate) mod sealed {
    use std::io::Write;

    pub trait Sealed {}

    impl<W: Write> Sealed for W {}
}

/// A chunk-value emission target.
///
/// `io::Write` implementors serialize each chunk immediately as bytes; other
/// sinks may route or buffer chunk values without going through an
/// intermediate byte encoding. This trait is sealed and cannot be
/// implemented outside this crate.
///
/// It is deliberately not re-exported at the crate root: the blanket
/// implementation below puts its methods on every [`Write`](io::Write) type, so
/// a root re-export would inject them into the method resolution of any crate
/// doing `use libpna::*`.
pub trait WriteChunk: sealed::Sealed {
    /// Emits `chunk` to this sink and returns the number of bytes written.
    ///
    /// # Errors
    ///
    /// Returns an error if the sink fails to accept the chunk.
    fn write_chunk<C: Chunk>(&mut self, chunk: C) -> io::Result<usize>;

    /// Consumes this sink, emits the archive-end marker, and returns the
    /// finalized sink.
    ///
    /// Prefer [`Archive::finalize`](crate::Archive::finalize) when constructing
    /// an archive so the marker is emitted at the correct point.
    ///
    /// # Errors
    ///
    /// Returns an error if the sink fails to write or finalize its output.
    #[inline]
    fn finalize_archive(mut self) -> io::Result<Self>
    where
        Self: Sized,
    {
        self.write_chunk((ChunkType::AEND, []))?;
        Ok(self)
    }

    /// Flushes any output buffered by this sink.
    ///
    /// Named `flush_chunks` rather than `flush` because the blanket
    /// implementation below also covers every [`Write`](io::Write) type;
    /// reusing `flush` would make the method ambiguous on such types wherever
    /// both traits are in scope.
    ///
    /// # Errors
    ///
    /// Returns an error if the sink fails to flush its buffered output.
    fn flush_chunks(&mut self) -> io::Result<()>;
}

impl<W: io::Write> WriteChunk for W {
    #[inline]
    fn write_chunk<C: Chunk>(&mut self, chunk: C) -> io::Result<usize> {
        crate::io::write_chunk(self, chunk)
    }

    #[inline]
    fn flush_chunks(&mut self) -> io::Result<()> {
        self.flush()
    }
}

/// Reads and validates the PNA archive signature.
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
pub fn read_signature<R: io::Read + ?Sized>(reader: &mut R) -> io::Result<()> {
    let mut signature = [0u8; PNA_SIGNATURE.len()];
    reader.read_exact(&mut signature)?;
    crate::format::validate_signature(&signature)
}

/// Reads and validates one PNA chunk from `reader`.
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
pub fn read_chunk<R: io::Read + ?Sized>(
    reader: &mut R,
    max_data_len: u32,
) -> io::Result<RawChunk<Vec<u8>>> {
    let mut length = [0u8; mem::size_of::<u32>()];
    reader.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length);
    if length > max_data_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("chunk data length {length} exceeds limit {max_data_len}"),
        ));
    }

    let mut ty = [0u8; mem::size_of::<ChunkType>()];
    reader.read_exact(&mut ty)?;
    let chunk_type = ChunkType::new(ty)?;

    let mut data = try_zeroed_vec(length as usize)?;
    reader.read_exact(&mut data)?;

    let mut crc = [0u8; mem::size_of::<u32>()];
    reader.read_exact(&mut crc)?;
    let crc = u32::from_be_bytes(crc);
    crate::format::validate_chunk_crc(&ty, &data, crc)?;

    Ok(RawChunk {
        length,
        ty: chunk_type,
        data,
        crc,
    })
}

/// Writes one PNA chunk to `writer`.
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
///
/// # Examples
///
/// ```
/// use libpna::{ChunkType, io};
///
/// let mut output = Vec::new();
/// let written = io::write_chunk(&mut output, (ChunkType::AEND, []))?;
///
/// assert_eq!(written, 12);
/// assert_eq!(output, [0, 0, 0, 0, b'A', b'E', b'N', b'D', 107, 246, 72, 109]);
/// # Ok::<(), std::io::Error>(())
/// ```
#[inline]
pub fn write_chunk<W: io::Write + ?Sized>(writer: &mut W, chunk: impl Chunk) -> io::Result<usize> {
    let length = chunk.length().to_be_bytes();
    let ty = chunk.ty();
    let data = chunk.data();
    let crc = chunk.crc().to_be_bytes();
    let bytes_len = MIN_CHUNK_BYTES_SIZE
        .checked_add(data.len())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "chunk byte length overflow"))?;

    writer.write_all(&length)?;
    writer.write_all(ty.as_bytes())?;
    writer.write_all(data)?;
    writer.write_all(&crc)?;
    Ok(bytes_len)
}

/// Skips one PNA chunk on `reader`.
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
pub fn skip_chunk<R: io::Read + io::Seek + ?Sized>(reader: &mut R) -> io::Result<(ChunkType, u64)> {
    let mut length = [0u8; mem::size_of::<u32>()];
    reader.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length);

    let mut ty = [0u8; mem::size_of::<ChunkType>()];
    reader.read_exact(&mut ty)?;
    let ty = ChunkType::new(ty)?;

    reader.seek_relative(length.into())?;

    let mut crc = [0u8; mem::size_of::<u32>()];
    reader.read_exact(&mut crc)?;

    Ok((ty, MIN_CHUNK_BYTES_SIZE as u64 + u64::from(length)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::test_support::{raw_chunk_bytes, valid_chunk_bytes};
    use crate::{Chunk, util::io::tests::PartialReader};
    use std::io::{Read, Seek};
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn read_signature_consumes_exactly_the_signature() {
        let input = [PNA_SIGNATURE.as_slice(), b"body"].concat();
        let mut reader = io::Cursor::new(input);
        read_signature(&mut reader).unwrap();
        assert_eq!(reader.position(), PNA_SIGNATURE.len() as u64);
    }

    #[test]
    fn read_signature_accepts_signature_split_across_reads() {
        let input = [PNA_SIGNATURE.as_slice(), b"body"].concat();
        let mut reader = PartialReader::new(input, [3u8, 2, 4]);
        read_signature(&mut reader).unwrap();
    }

    #[test]
    fn read_signature_rejects_input_one_byte_short() {
        let mut reader = io::Cursor::new(&PNA_SIGNATURE[..7]);
        assert_eq!(
            read_signature(&mut reader).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn read_signature_rejects_mismatched_signature() {
        let mut reader = io::Cursor::new(b"xxxxxxxx");
        assert_eq!(
            read_signature(&mut reader).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn read_chunk_returns_owned_chunk_and_stops_at_its_crc() {
        let mut input = valid_chunk_bytes();
        let chunk_len = input.len();
        input.extend_from_slice(b"following");
        let mut reader = io::Cursor::new(input);

        let chunk = read_chunk(&mut reader, u32::MAX).unwrap();

        assert_eq!(chunk.ty(), ChunkType::FDAT);
        assert_eq!(chunk.data(), [0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(reader.position(), chunk_len as u64);
    }

    #[test]
    fn read_chunk_accepts_fields_split_across_reads() {
        let bytes = valid_chunk_bytes();
        let mut reader = PartialReader::new(bytes, [1u8; 32]);
        assert_eq!(read_chunk(&mut reader, u32::MAX).unwrap().length(), 4);
    }

    #[test]
    fn read_chunk_enforces_inclusive_data_length_limit() {
        let mut reader = io::Cursor::new(raw_chunk_bytes(*b"AEND", &[]));
        let chunk = read_chunk(&mut reader, 0).unwrap();
        assert_eq!(chunk.ty(), ChunkType::AEND);
        assert!(chunk.data().is_empty());

        let mut reader = io::Cursor::new(valid_chunk_bytes());
        assert_eq!(read_chunk(&mut reader, 4).unwrap().length(), 4);

        let mut reader = io::Cursor::new(valid_chunk_bytes());
        assert_eq!(
            read_chunk(&mut reader, 3).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn read_chunk_checks_limit_before_requiring_the_remaining_fields() {
        let mut reader = io::Cursor::new(u32::MAX.to_be_bytes());
        assert_eq!(
            read_chunk(&mut reader, 1024).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn read_chunk_rejects_every_truncation_boundary() {
        let bytes = valid_chunk_bytes();
        for end in 0..bytes.len() {
            let mut reader = io::Cursor::new(&bytes[..end]);
            assert_eq!(
                read_chunk(&mut reader, u32::MAX).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "truncation at byte {end}",
            );
        }
    }

    #[test]
    fn read_chunk_applies_chunk_type_validation_rules() {
        let mut reader = io::Cursor::new(raw_chunk_bytes(*b"FD1T", b"data"));
        assert_eq!(
            read_chunk(&mut reader, u32::MAX).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        let mut reader = io::Cursor::new(raw_chunk_bytes(*b"ABcD", b"data"));
        assert_eq!(read_chunk(&mut reader, u32::MAX).unwrap().data(), b"data");
    }

    #[test]
    fn read_chunk_rejects_crc_mismatch() {
        let mut bytes = valid_chunk_bytes();
        *bytes.last_mut().unwrap() ^= 0xFF;
        let mut reader = io::Cursor::new(bytes);
        assert_eq!(
            read_chunk(&mut reader, u32::MAX).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn write_chunk_preserves_reported_fields() {
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

        let mut output = Vec::new();
        let written = write_chunk(&mut output, StoredChunk).unwrap();

        assert_eq!(written, 15);
        assert_eq!(
            output,
            [
                0, 0, 0, 1, b'F', b'D', b'A', b'T', b'a', b'b', b'c', 1, 2, 3, 4
            ]
        );
    }

    struct CountingReader<R> {
        inner: R,
        read_bytes: usize,
        seeks: usize,
    }

    impl<R: Read> Read for CountingReader<R> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let n = self.inner.read(buf)?;
            self.read_bytes += n;
            Ok(n)
        }
    }

    impl<R: Seek> Seek for CountingReader<R> {
        fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
            self.seeks += 1;
            self.inner.seek(pos)
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

    impl Read for RecordingSeekReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.pos < self.header.len() {
                let n = buf.len().min(self.header.len() - self.pos);
                buf[..n].copy_from_slice(&self.header[self.pos..self.pos + n]);
                self.pos += n;
                Ok(n)
            } else if self.trailer_left > 0 {
                let n = buf.len().min(self.trailer_left);
                buf[..n].fill(0);
                self.trailer_left -= n;
                Ok(n)
            } else {
                Ok(0)
            }
        }
    }

    impl Seek for RecordingSeekReader {
        fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
            self.seek_from = Some(pos);
            Ok(0)
        }
    }

    #[test]
    fn skip_chunk_returns_type_and_size_without_reading_data() {
        let mut input = valid_chunk_bytes();
        let chunk_len = input.len();
        input.extend_from_slice(b"following");
        let mut reader = CountingReader {
            inner: io::Cursor::new(input),
            read_bytes: 0,
            seeks: 0,
        };

        let (ty, consumed) = skip_chunk(&mut reader).unwrap();

        assert_eq!(ty, ChunkType::FDAT);
        assert_eq!(consumed, chunk_len as u64);
        assert_eq!(reader.inner.position(), consumed);
        assert_eq!(reader.read_bytes, MIN_CHUNK_BYTES_SIZE);
    }

    #[test]
    fn skip_chunk_seeks_over_the_maximum_data_length() {
        let mut reader = RecordingSeekReader {
            header: [&u32::MAX.to_be_bytes()[..], b"FDAT"].concat(),
            pos: 0,
            trailer_left: 4,
            seek_from: None,
        };
        let (ty, consumed) = skip_chunk(&mut reader).unwrap();
        assert_eq!(ty, ChunkType::FDAT);
        assert_eq!(consumed, u64::from(u32::MAX) + MIN_CHUNK_BYTES_SIZE as u64);
        assert_eq!(
            reader.seek_from,
            Some(io::SeekFrom::Current(i64::from(u32::MAX)))
        );
    }

    #[test]
    fn skip_chunk_rejects_invalid_chunk_type_before_seeking() {
        let mut reader = io::Cursor::new([&4u32.to_be_bytes()[..], b"FD1T"].concat());
        assert_eq!(
            skip_chunk(&mut reader).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        assert_eq!(reader.position(), 8);
    }

    #[test]
    fn skip_chunk_rejects_every_truncation_boundary() {
        let bytes = valid_chunk_bytes();
        for end in 0..bytes.len() {
            let mut reader = io::Cursor::new(&bytes[..end]);
            assert_eq!(
                skip_chunk(&mut reader).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "truncation at byte {end}",
            );
        }
    }

    #[test]
    fn skip_chunk_handles_empty_data() {
        let mut reader = io::Cursor::new(raw_chunk_bytes(*b"AEND", &[]));
        let (ty, consumed) = skip_chunk(&mut reader).unwrap();
        assert_eq!(ty, ChunkType::AEND);
        assert_eq!(consumed, MIN_CHUNK_BYTES_SIZE as u64);
    }

    #[test]
    fn skip_chunk_does_not_validate_crc() {
        let mut bytes = valid_chunk_bytes();
        *bytes.last_mut().unwrap() ^= 0xFF;
        let mut reader = io::Cursor::new(bytes);
        assert_eq!(skip_chunk(&mut reader).unwrap().0, ChunkType::FDAT);
    }

    #[test]
    fn skip_chunk_walks_consecutive_chunks_through_a_buf_reader() {
        let mut input = valid_chunk_bytes();
        input.extend_from_slice(&raw_chunk_bytes(*b"AEND", &[]));
        let total = input.len() as u64;
        let mut reader = io::BufReader::with_capacity(
            64,
            CountingReader {
                inner: io::Cursor::new(input),
                read_bytes: 0,
                seeks: 0,
            },
        );
        assert_eq!(skip_chunk(&mut reader).unwrap().0, ChunkType::FDAT);
        assert_eq!(skip_chunk(&mut reader).unwrap().0, ChunkType::AEND);
        assert_eq!(reader.get_ref().seeks, 0);
        assert_eq!(reader.stream_position().unwrap(), total);
    }
}
