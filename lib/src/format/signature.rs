//! PNA signature definition and validation.

use std::io;

/// The signature at the beginning of every PNA archive.
pub const PNA_SIGNATURE: &[u8; 8] = b"\x89PNA\r\n\x1A\n";

/// Validates an exact-length PNA signature.
#[inline]
pub(crate) fn validate_signature(signature: &[u8; PNA_SIGNATURE.len()]) -> io::Result<()> {
    if signature != PNA_SIGNATURE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "not a PNA archive",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn validate_signature_accepts_pna_signature() {
        validate_signature(PNA_SIGNATURE).unwrap();
    }

    #[test]
    fn validate_signature_rejects_mismatched_signature() {
        assert_eq!(
            validate_signature(b"xxxxxxxx").unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
