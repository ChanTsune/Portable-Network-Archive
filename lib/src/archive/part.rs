//! Continuation across the parts of a multipart archive.
//!
//! A PNA archive may be split across several parts, each self-framed and, when
//! it is not the last, closed by `ANXT` followed by `AEND`. Reading past that
//! boundary means obtaining the next part from somewhere the archive itself
//! cannot know about - a file next to the current one, a network range request,
//! a buffer already in memory. [`PartProvider`] is that seam.
use std::io::{self, Read};

/// Supplies the next physical part of a multipart archive.
pub trait PartProvider<R: Read> {
    /// Opens the part whose AHED archive number is expected to equal `expected`.
    ///
    /// Returning `Ok(None)` reports that the required part is unavailable and
    /// fails the current cursor rather than implicitly retrying it.
    ///
    /// # Errors
    ///
    /// Returns any error encountered while locating or opening the part.
    fn next_part(&mut self, expected: u32) -> io::Result<Option<R>>;
}

/// Marker provider used internally for a single physical archive.
///
/// Values of this type cannot be constructed outside this crate.
#[derive(Clone, Copy, Debug)]
pub struct NoParts {
    _private: (),
}

impl NoParts {
    pub(crate) const NEW: Self = Self { _private: () };
}

impl<R: Read> PartProvider<R> for NoParts {
    #[inline]
    fn next_part(&mut self, _expected: u32) -> io::Result<Option<R>> {
        Ok(None)
    }
}
