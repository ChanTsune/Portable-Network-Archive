//! Asynchronous I/O primitives for reading and writing PNA archives.

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

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;
    use futures_util::io::Cursor;

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
