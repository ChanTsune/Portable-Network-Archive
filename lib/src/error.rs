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
/// The three variants correspond to the distinct failure classes of the PNA
/// specification: structural violations of the datastream layout, GCM tag
/// verification failures, and truncated streams.
#[derive(Debug)]
#[non_exhaustive]
pub(crate) enum AeadError {
    /// The datastream violates the AEAD layout (e.g. invalid segment size,
    /// short non-final segment, or a stream shorter than the minimum length).
    Malformed(&'static str),
    /// A GCM authentication tag did not verify. A wrong password and data
    /// tampering are cryptographically indistinguishable causes.
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
            Self::AuthenticationFailure => {
                f.write_str("authentication failed: wrong password or corrupted data")
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
    fn display_malformed() {
        let err = AeadError::Malformed("invalid segment size");
        assert_eq!(
            err.to_string(),
            "malformed AEAD datastream: invalid segment size"
        );
    }

    #[test]
    fn display_authentication_failure() {
        let err = AeadError::AuthenticationFailure;
        assert_eq!(
            err.to_string(),
            "authentication failed: wrong password or corrupted data"
        );
    }

    #[test]
    fn display_truncation() {
        let err = AeadError::Truncation;
        assert_eq!(err.to_string(), "AEAD datastream is truncated");
    }

    #[test]
    fn into_io_error_malformed() {
        let err = AeadError::Malformed("test");
        let io_err: std::io::Error = err.into();
        assert_eq!(io_err.kind(), std::io::ErrorKind::InvalidData);
        assert!(io_err.get_ref().is_some());
    }

    #[test]
    fn into_io_error_authentication_failure() {
        let err = AeadError::AuthenticationFailure;
        let io_err: std::io::Error = err.into();
        assert_eq!(io_err.kind(), std::io::ErrorKind::InvalidData);
        assert!(io_err.get_ref().is_some());
    }

    #[test]
    fn into_io_error_truncation() {
        let err = AeadError::Truncation;
        let io_err: std::io::Error = err.into();
        assert_eq!(io_err.kind(), std::io::ErrorKind::InvalidData);
        assert!(io_err.get_ref().is_some());
    }

    #[test]
    fn downcast_malformed() {
        let err = AeadError::Malformed("test");
        let io_err: std::io::Error = err.into();
        let recovered = io_err.get_ref().and_then(|e| e.downcast_ref::<AeadError>());
        assert!(recovered.is_some());
        assert!(matches!(recovered.unwrap(), AeadError::Malformed("test")));
    }

    #[test]
    fn downcast_authentication_failure() {
        let err = AeadError::AuthenticationFailure;
        let io_err: std::io::Error = err.into();
        let recovered = io_err.get_ref().and_then(|e| e.downcast_ref::<AeadError>());
        assert!(recovered.is_some());
        assert!(matches!(
            recovered.unwrap(),
            AeadError::AuthenticationFailure
        ));
    }

    #[test]
    fn downcast_truncation() {
        let err = AeadError::Truncation;
        let io_err: std::io::Error = err.into();
        let recovered = io_err.get_ref().and_then(|e| e.downcast_ref::<AeadError>());
        assert!(recovered.is_some());
        assert!(matches!(recovered.unwrap(), AeadError::Truncation));
    }
}
