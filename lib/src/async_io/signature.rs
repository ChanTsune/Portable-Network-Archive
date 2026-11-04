use crate::PNA_SIGNATURE;
use futures_io::AsyncRead;
use futures_util::AsyncReadExt;
use std::io;

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
#[inline]
pub async fn is_pna<R: AsyncRead + Unpin + ?Sized>(reader: &mut R) -> io::Result<bool> {
    let mut signature = [0u8; PNA_SIGNATURE.len()];
    match reader.read_exact(&mut signature).await {
        Ok(()) => Ok(crate::format::validate_signature(&signature).is_ok()),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use futures_util::io::Cursor;

    #[tokio::test]
    async fn is_pna_accepts_signature() {
        let mut reader = Cursor::new(PNA_SIGNATURE.to_vec());
        assert!(is_pna(&mut reader).await.unwrap());
        assert_eq!(reader.position(), PNA_SIGNATURE.len() as u64);

        let input = [PNA_SIGNATURE.as_slice(), b"body"].concat();
        let mut reader = Cursor::new(input);
        assert!(is_pna(&mut reader).await.unwrap());
        assert_eq!(reader.position(), PNA_SIGNATURE.len() as u64);
    }

    #[tokio::test]
    async fn is_pna_rejects_non_signature() {
        let mut reader = Cursor::new(b"xxxxxxxx");
        assert!(!is_pna(&mut reader).await.unwrap());

        let mut tampered = *PNA_SIGNATURE;
        tampered[0] ^= 0xFF;
        let mut reader = Cursor::new(tampered.to_vec());
        assert!(!is_pna(&mut reader).await.unwrap());
    }

    #[tokio::test]
    async fn is_pna_reports_short_input_as_false() {
        for len in 0..PNA_SIGNATURE.len() {
            let mut reader = Cursor::new(&PNA_SIGNATURE[..len]);
            assert!(
                !is_pna(&mut reader).await.unwrap(),
                "truncation at {len} bytes must be false",
            );
        }
    }

    #[tokio::test]
    async fn is_pna_matches_bytes_verdict_for_representative_inputs() {
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
            let mut reader = Cursor::new(input);
            assert_eq!(
                is_pna(&mut reader).await.unwrap(),
                crate::bytes::is_pna(input),
                "verdict differs for {input:?}",
            );
        }
    }

    #[tokio::test]
    async fn is_pna_propagates_reader_errors() {
        use std::pin::Pin;
        use std::task::{Context, Poll};
        struct FailingReader(io::ErrorKind);
        impl AsyncRead for FailingReader {
            fn poll_read(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                _buf: &mut [u8],
            ) -> Poll<io::Result<usize>> {
                Poll::Ready(Err(io::Error::new(self.0, "reader failure")))
            }
        }
        let mut reader = FailingReader(io::ErrorKind::InvalidData);
        assert_eq!(
            is_pna(&mut reader).await.unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let mut reader = FailingReader(io::ErrorKind::Other);
        assert_eq!(
            is_pna(&mut reader).await.unwrap_err().kind(),
            io::ErrorKind::Other
        );
    }

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
}
