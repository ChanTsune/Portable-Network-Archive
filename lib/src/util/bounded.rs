//! Byte-length-bounded owned UTF-8 strings and byte slices.
//!
//! [`str::BoundedString`] and [`bytes::BoundedBytes`] reject values whose byte
//! length exceeds their `MAX` bound.
//!
//! # Examples
//!
//! ```ignore
//! // A field whose on-the-wire length prefix is a `u8` accepts at most 255 bytes.
//! let name: BoundedString<255> = "root".try_into().unwrap();
//! assert_eq!(name.len(), 4);
//!
//! let too_long: Result<BoundedString<255>, _> = "a".repeat(256).try_into();
//! assert!(too_long.is_err());
//!
//! // Arbitrary bytes (non-UTF-8 permitted).
//! let payload: BoundedBytes<8> = vec![0xFF, 0x00, 0x42].try_into().unwrap();
//! assert_eq!(payload.len(), 3);
//! ```

pub mod bytes;
pub mod str;

use std::{error, fmt};

/// Error returned when a value exceeds the byte-length bound of a bounded
/// owned string or byte slice.
///
/// # Examples
///
/// ```ignore
/// let err = BoundedString::<3>::new("hello").unwrap_err();
/// assert_eq!(err.max(), 3);
/// assert_eq!(err.actual(), 5);
/// ```
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub struct LengthExceeded {
    max: usize,
    actual: usize,
}

impl LengthExceeded {
    pub(crate) const fn new(max: usize, actual: usize) -> Self {
        Self { max, actual }
    }

    /// Returns the maximum byte length permitted by the bounding type.
    #[inline]
    #[must_use]
    pub const fn max(&self) -> usize {
        self.max
    }

    /// Returns the actual byte length of the rejected input.
    #[inline]
    #[must_use]
    pub const fn actual(&self) -> usize {
        self.actual
    }
}

impl fmt::Display for LengthExceeded {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "length {} exceeds bound {}", self.actual, self.max)
    }
}

impl error::Error for LengthExceeded {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_formats_actual_then_bound() {
        let err = LengthExceeded::new(255, 256);
        assert_eq!(format!("{err}"), "length 256 exceeds bound 255");
    }
}
