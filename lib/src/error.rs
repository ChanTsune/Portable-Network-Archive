//! Error types for PNA archive operations.

use std::{
    error::Error,
    fmt::{Display, Formatter},
};

/// Unknown value error.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct UnknownValueError(pub(crate) u8);

impl Display for UnknownValueError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown value {}", self.0)
    }
}

impl Error for UnknownValueError {}

/// Error kinds reported while decrypting an AEAD (Cipher mode 2) datastream.
///
/// The variants correspond one-to-one with the failure classes the PNA
/// specification requires a decoder to report distinctly.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) enum AeadError {
    /// The datastream violates the AEAD layout; the detail names which rule.
    Malformed(&'static str),
    /// The stream header's key confirmation does not match the key derived
    /// from the password, so the password does not match the recorded KDF
    /// parameters. Tampering with the `PHSF` chunk or with the key
    /// confirmation field itself presents the same way. Reported before any
    /// segment is processed.
    KeyMismatch,
    /// A GCM authentication tag did not verify: the datastream has been
    /// tampered with or corrupted. A wrong password is reported as
    /// [`AeadError::KeyMismatch`] before segment processing starts, so it
    /// cannot reach here.
    AuthenticationFailure,
    /// The datastream ended with a partial tail too short to be a final
    /// segment, after at least one verified segment. A truncation that leaves
    /// a plausible final segment is reported as
    /// [`AeadError::AuthenticationFailure`] instead, since GCM cannot
    /// distinguish it from tampering.
    Truncation,
}

impl std::fmt::Display for AeadError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(detail) => write!(f, "malformed AEAD datastream: {detail}"),
            Self::KeyMismatch => f.write_str(
                "key mismatch: the password does not match the recorded key derivation parameters",
            ),
            Self::AuthenticationFailure => {
                f.write_str("authentication failed: the AEAD datastream is corrupted or tampered")
            }
            Self::Truncation => f.write_str("AEAD datastream is truncated"),
        }
    }
}

impl std::error::Error for AeadError {}

impl From<AeadError> for std::io::Error {
    #[inline]
    fn from(e: AeadError) -> Self {
        // Deliberately not `UnexpectedEof` even for `Truncation`: readers treat
        // `UnexpectedEof` as a clean end of stream, which would let a truncated
        // authenticated datastream terminate iteration without an error.
        std::io::Error::new(std::io::ErrorKind::InvalidData, e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_does_not_become_unexpected_eof() {
        let io_err: std::io::Error = AeadError::Truncation.into();
        assert_eq!(io_err.kind(), std::io::ErrorKind::InvalidData);
    }
}
