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

/// Checks whether the bytes at the start of `reader` are a PNA signature.
///
/// On `Ok(true)`, `reader` has consumed exactly the signature bytes. On
/// `Ok(false)` or `Err`, an unspecified number of bytes has been consumed, so
/// `reader` cannot be reused to probe for another format.
///
/// A matching signature does not imply that the archive is complete or valid.
///
/// If fewer bytes than the signature could be read, this returns `Ok(false)`.
///
/// Use [`crate::bytes::is_pna`] when the input is already in memory and you
/// want a non-consuming check.
///
/// # Errors
///
/// Propagates any error produced by `reader`, except that a short read
/// ([`io::ErrorKind::UnexpectedEof`]) is reported as `Ok(false)` instead.
/// Signature mismatch is reported as `Ok(false)`; this function does not
/// itself return [`io::ErrorKind::InvalidData`], but an `InvalidData` error
/// produced by `reader` propagates.
///
/// # Examples
///
/// ```
/// use std::io::Cursor;
///
/// let mut reader = Cursor::new(libpna::PNA_SIGNATURE.to_vec());
/// assert!(libpna::io::is_pna(&mut reader)?);
///
/// let mut reader = Cursor::new(b"not a pna archive");
/// assert!(!libpna::io::is_pna(&mut reader)?);
///
/// let n = libpna::PNA_SIGNATURE.len();
/// let mut reader = Cursor::new(&libpna::PNA_SIGNATURE[..n - 1]);
/// assert!(!libpna::io::is_pna(&mut reader)?);
/// # Ok::<(), std::io::Error>(())
/// ```
#[inline]
pub fn is_pna<R: io::Read + ?Sized>(reader: &mut R) -> io::Result<bool> {
    let mut signature = [0u8; PNA_SIGNATURE.len()];
    match reader.read_exact(&mut signature) {
        Ok(()) => Ok(crate::format::validate_signature(&signature).is_ok()),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::io::tests::PartialReader;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn is_pna_accepts_signature() {
        let mut input = io::Cursor::new(PNA_SIGNATURE.to_vec());
        assert!(is_pna(&mut input).unwrap());
        assert_eq!(input.position(), PNA_SIGNATURE.len() as u64);

        let input = [PNA_SIGNATURE.as_slice(), b"body"].concat();
        let mut reader = io::Cursor::new(input);
        assert!(is_pna(&mut reader).unwrap());
        assert_eq!(reader.position(), PNA_SIGNATURE.len() as u64);
    }

    #[test]
    fn is_pna_rejects_non_signature() {
        let mut input = io::Cursor::new([b'x'; PNA_SIGNATURE.len()]);
        assert!(!is_pna(&mut input).unwrap());

        let mut tampered = *PNA_SIGNATURE;
        tampered[0] ^= 0xFF;
        let mut input = io::Cursor::new(tampered);
        assert!(!is_pna(&mut input).unwrap());
    }

    #[test]
    fn is_pna_reports_short_input_as_false() {
        for len in 0..PNA_SIGNATURE.len() {
            let mut input = io::Cursor::new(&PNA_SIGNATURE[..len]);
            assert!(
                !is_pna(&mut input).unwrap(),
                "truncation at {len} bytes must be false",
            );
        }
    }

    #[test]
    fn is_pna_matches_bytes_verdict_for_representative_inputs() {
        let mut tampered = *PNA_SIGNATURE;
        tampered[0] ^= 0xFF;
        let cases: Vec<Vec<u8>> = vec![
            PNA_SIGNATURE.to_vec(),
            [PNA_SIGNATURE.as_slice(), b"body"].concat(),
            b"xxxxxxxx".to_vec(),
            tampered.to_vec(),
            PNA_SIGNATURE[..PNA_SIGNATURE.len() - 1].to_vec(),
        ];
        for input in &cases {
            let mut reader = io::Cursor::new(input);
            assert_eq!(
                is_pna(&mut reader).unwrap(),
                crate::bytes::is_pna(input),
                "verdict differs for {input:?}",
            );
        }
    }

    #[test]
    fn is_pna_propagates_reader_errors() {
        struct FailingReader(io::ErrorKind);
        impl io::Read for FailingReader {
            fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(self.0, "reader failure"))
            }
        }
        // Reader-originated `InvalidData` must propagate, not become `Ok(false)`.
        let mut reader = FailingReader(io::ErrorKind::InvalidData);
        assert_eq!(
            is_pna(&mut reader).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let mut reader = FailingReader(io::ErrorKind::Other);
        assert_eq!(
            is_pna(&mut reader).unwrap_err().kind(),
            io::ErrorKind::Other
        );
    }

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
