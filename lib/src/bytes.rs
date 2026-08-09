//! Byte-slice primitives for parsing PNA archives.

use crate::PNA_SIGNATURE;
use std::io;

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

#[cfg(test)]
mod tests {
    use super::*;
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
}
