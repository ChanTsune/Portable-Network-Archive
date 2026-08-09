//! PNA chunk integrity calculation and validation.

use std::io;

/// Calculates the CRC-32 checksum of a chunk type and data field.
#[inline]
pub(crate) fn chunk_crc(ty: &[u8; 4], data: &[u8]) -> u32 {
    let mut crc = crc32fast::Hasher::new();
    crc.update(ty);
    crc.update(data);
    crc.finalize()
}

/// Validates a stored chunk CRC-32 checksum.
#[inline]
pub(crate) fn validate_chunk_crc(ty: &[u8; 4], data: &[u8], stored: u32) -> io::Result<()> {
    if stored != chunk_crc(ty, data) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "broken chunk"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn calculates_known_chunk_crc() {
        assert_eq!(chunk_crc(b"FDAT", &[0xAA, 0xBB, 0xCC, 0xDD]), 0x47F3_2B10);
    }

    #[test]
    fn validates_matching_chunk_crc() {
        validate_chunk_crc(b"FDAT", &[0xAA, 0xBB, 0xCC, 0xDD], 0x47F3_2B10).unwrap();
    }

    #[test]
    fn rejects_mismatched_chunk_crc() {
        assert_eq!(
            validate_chunk_crc(b"FDAT", &[0xAA, 0xBB, 0xCC, 0xDD], 0)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }
}
