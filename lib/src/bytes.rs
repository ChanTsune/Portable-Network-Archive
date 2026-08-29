//! Byte-slice primitives for parsing PNA archives.

use crate::{ChunkType, PNA_SIGNATURE, RawChunk};
use std::{io, mem};

/// Reads and validates the PNA archive signature from `bytes`.
///
/// Returns the bytes following the signature on success.
///
/// # Errors
///
/// Returns [`io::ErrorKind::UnexpectedEof`] when the signature is incomplete,
/// or [`io::ErrorKind::InvalidData`] when it does not match.
#[inline]
pub fn read_signature(bytes: &[u8]) -> io::Result<&[u8]> {
    let (signature, rest) = bytes
        .split_first_chunk::<{ PNA_SIGNATURE.len() }>()
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    crate::format::validate_signature(signature)?;
    Ok(rest)
}

/// Reads and validates one PNA chunk from the beginning of `bytes`.
///
/// Returns the zero-copy chunk and the bytes following it. The input must start
/// at the chunk length field; this function does not read the archive signature
/// or interpret archive-level chunk ordering.
///
/// `max_data_len` is an inclusive upper bound for the chunk data length. Pass
/// [`u32::MAX`] to allow the full range representable by the PNA format.
///
/// # Errors
///
/// Returns [`io::ErrorKind::UnexpectedEof`] when any chunk field is incomplete,
/// or [`io::ErrorKind::InvalidData`] when the declared data length exceeds
/// `max_data_len`, the chunk type is invalid, or the stored CRC-32 does not
/// match the chunk type and data.
#[inline]
pub fn read_chunk(bytes: &[u8], max_data_len: u32) -> io::Result<(RawChunk<&[u8]>, &[u8])> {
    let (length, bytes) = bytes
        .split_first_chunk::<{ mem::size_of::<u32>() }>()
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    let length = u32::from_be_bytes(*length);
    if length > max_data_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("chunk data length {length} exceeds limit {max_data_len}"),
        ));
    }

    let (ty, bytes) = bytes
        .split_first_chunk::<{ mem::size_of::<ChunkType>() }>()
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    let chunk_type = ChunkType::new(*ty)?;

    let (data, bytes) = bytes
        .split_at_checked(length as usize)
        .ok_or(io::ErrorKind::UnexpectedEof)?;

    let (crc, bytes) = bytes
        .split_first_chunk::<{ mem::size_of::<u32>() }>()
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    let crc = u32::from_be_bytes(*crc);
    crate::format::validate_chunk_crc(ty, data, crc)?;

    Ok((
        RawChunk {
            length,
            ty: chunk_type,
            data,
            crc,
        },
        bytes,
    ))
}

/// Skips one PNA chunk at the beginning of `bytes`.
///
/// Returns the chunk type and the bytes following the chunk. The input must
/// start at the chunk length field. The chunk data and CRC are skipped based
/// on the declared data length and are not validated. This function does not
/// read the archive signature or interpret archive-level chunk ordering.
///
/// # Errors
///
/// Returns [`io::ErrorKind::UnexpectedEof`] when any chunk field does not fit
/// in `bytes`, or [`io::ErrorKind::InvalidData`] when the chunk type is
/// invalid.
#[inline]
pub fn skip_chunk(bytes: &[u8]) -> io::Result<(ChunkType, &[u8])> {
    let (length, bytes) = bytes
        .split_first_chunk::<{ mem::size_of::<u32>() }>()
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    let length = u32::from_be_bytes(*length);

    let (ty, bytes) = bytes
        .split_first_chunk::<{ mem::size_of::<ChunkType>() }>()
        .ok_or(io::ErrorKind::UnexpectedEof)?;
    let ty = ChunkType::new(*ty)?;

    let (_, bytes) = bytes
        .split_at_checked(length as usize)
        .ok_or(io::ErrorKind::UnexpectedEof)?;

    let (_, bytes) = bytes
        .split_first_chunk::<{ mem::size_of::<u32>() }>()
        .ok_or(io::ErrorKind::UnexpectedEof)?;

    Ok((ty, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Chunk;
    use crate::chunk::test_support::{raw_chunk_bytes, valid_chunk_bytes};
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn read_signature_returns_remaining_bytes() {
        let input = [PNA_SIGNATURE.as_slice(), b"body"].concat();
        assert_eq!(read_signature(&input).unwrap(), b"body");
        assert_eq!(read_signature(PNA_SIGNATURE).unwrap(), b"");
    }

    #[test]
    fn read_signature_rejects_input_one_byte_short() {
        assert_eq!(
            read_signature(&PNA_SIGNATURE[..7]).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn read_signature_rejects_mismatched_signature() {
        assert_eq!(
            read_signature(b"xxxxxxxx").unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn read_chunk_returns_borrowed_chunk_and_remainder() {
        let mut input = valid_chunk_bytes();
        input.extend_from_slice(b"following");

        let (chunk, rest) = read_chunk(&input, u32::MAX).unwrap();

        assert_eq!(chunk.ty(), ChunkType::FDAT);
        assert_eq!(chunk.data(), [0xAA, 0xBB, 0xCC, 0xDD]);
        assert_eq!(chunk.data().as_ptr(), input[8..].as_ptr());
        assert_eq!(rest, b"following");
    }

    #[test]
    fn read_chunk_enforces_inclusive_data_length_limit() {
        let bytes = raw_chunk_bytes(*b"AEND", &[]);
        let (chunk, rest) = read_chunk(&bytes, 0).unwrap();
        assert_eq!(chunk.ty(), ChunkType::AEND);
        assert!(chunk.data().is_empty());
        assert!(rest.is_empty());

        let bytes = valid_chunk_bytes();
        assert_eq!(read_chunk(&bytes, 4).unwrap().0.length(), 4);

        let bytes = valid_chunk_bytes();
        assert_eq!(
            read_chunk(&bytes, 3).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn read_chunk_checks_limit_before_requiring_the_remaining_fields() {
        assert_eq!(
            read_chunk(&u32::MAX.to_be_bytes(), 1024)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn read_chunk_rejects_every_truncation_boundary() {
        let bytes = valid_chunk_bytes();
        for end in 0..bytes.len() {
            assert_eq!(
                read_chunk(&bytes[..end], u32::MAX).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "truncation at byte {end}",
            );
        }
    }

    #[test]
    fn read_chunk_applies_chunk_type_validation_rules() {
        let bytes = raw_chunk_bytes(*b"FD1T", b"data");
        assert_eq!(
            read_chunk(&bytes, u32::MAX).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        let bytes = raw_chunk_bytes(*b"ABcD", b"data");
        assert_eq!(read_chunk(&bytes, u32::MAX).unwrap().0.data(), b"data");
    }

    #[test]
    fn read_chunk_rejects_crc_mismatch() {
        let mut bytes = valid_chunk_bytes();
        *bytes.last_mut().unwrap() ^= 0xFF;
        assert_eq!(
            read_chunk(&bytes, u32::MAX).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn skip_chunk_returns_type_and_remainder() {
        let mut input = valid_chunk_bytes();
        input.extend_from_slice(b"following");

        let (ty, rest) = skip_chunk(&input).unwrap();

        assert_eq!(ty, ChunkType::FDAT);
        assert_eq!(rest, b"following");
        assert_eq!(rest.as_ptr(), input[16..].as_ptr());
        assert_eq!(input.len() - rest.len(), 16);
    }

    #[test]
    fn skip_chunk_reports_eof_when_the_data_is_missing() {
        let input = [&u32::MAX.to_be_bytes()[..], b"FDAT"].concat();
        assert_eq!(
            skip_chunk(&input).unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn skip_chunk_rejects_invalid_chunk_type_before_checking_the_data() {
        let input = [&4u32.to_be_bytes()[..], b"FD1T"].concat();
        assert_eq!(
            skip_chunk(&input).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn skip_chunk_rejects_every_truncation_boundary() {
        let bytes = valid_chunk_bytes();
        for end in 0..bytes.len() {
            assert_eq!(
                skip_chunk(&bytes[..end]).unwrap_err().kind(),
                io::ErrorKind::UnexpectedEof,
                "truncation at byte {end}",
            );
        }
    }

    #[test]
    fn skip_chunk_handles_empty_data() {
        let bytes = raw_chunk_bytes(*b"AEND", &[]);
        let (ty, rest) = skip_chunk(&bytes).unwrap();
        assert_eq!(ty, ChunkType::AEND);
        assert!(rest.is_empty());
    }

    #[test]
    fn skip_chunk_does_not_validate_crc() {
        let mut bytes = valid_chunk_bytes();
        *bytes.last_mut().unwrap() ^= 0xFF;
        assert_eq!(skip_chunk(&bytes).unwrap().0, ChunkType::FDAT);
    }
}
