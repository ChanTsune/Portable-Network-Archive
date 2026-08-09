//! I/O primitives for reading and writing PNA archives.

use crate::PNA_SIGNATURE;
use std::io;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::io::tests::PartialReader;
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
}
