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

/// Checks whether `bytes` starts with a PNA signature.
///
/// Use this for in-memory probing. For streaming readers, see
/// [`crate::io::is_pna`].
///
/// Returns `false` for inputs shorter than the signature (including empty
/// input).
///
/// A matching signature does not imply that the archive is complete or valid.
///
/// # Examples
///
/// ```
/// assert!(libpna::bytes::is_pna(libpna::PNA_SIGNATURE));
/// assert!(!libpna::bytes::is_pna(b""));
/// assert!(!libpna::bytes::is_pna(b"not a pna archive"));
/// assert!(!libpna::bytes::is_pna(
///     &libpna::PNA_SIGNATURE[..libpna::PNA_SIGNATURE.len() - 1]
/// ));
/// ```
#[inline]
pub fn is_pna(bytes: &[u8]) -> bool {
    bytes
        .split_first_chunk::<{ PNA_SIGNATURE.len() }>()
        .is_some_and(|(signature, _)| crate::format::validate_signature(signature).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn is_pna_accepts_signature() {
        assert!(is_pna(PNA_SIGNATURE));
        assert!(is_pna(
            [PNA_SIGNATURE.as_slice(), b"body"].concat().as_slice()
        ));
    }

    #[test]
    fn is_pna_rejects_non_signature() {
        assert!(!is_pna(b"xxxxxxxx"));

        let mut tampered = *PNA_SIGNATURE;
        tampered[0] ^= 0xFF;
        assert!(!is_pna(&tampered));

        for len in 0..PNA_SIGNATURE.len() {
            assert!(
                !is_pna(&PNA_SIGNATURE[..len]),
                "truncation at {len} bytes must be false",
            );
        }
    }

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
