use std::{
    error::Error,
    fmt::{Display, Formatter},
};

/// Represents an error for an unknown or unsupported value.
///
/// This error is typically used when parsing data that contains a value not
/// recognized by the current implementation, such as an unknown compression
/// or encryption algorithm identifier.
///
/// The inner value is the byte that was not recognized.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct UnknownValueError(pub(crate) u8);

impl Display for UnknownValueError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown value {}", self.0)
    }
}

impl Error for UnknownValueError {}
