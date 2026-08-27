//! Continuation across the parts of a multipart archive.
//!
//! A PNA archive may be split across several parts, each self-framed and, when
//! it is not the last, closed by `ANXT` followed by `AEND`. Reading past that
//! boundary means obtaining the next part from somewhere the archive itself
//! cannot know about - a file next to the current one, a network range request,
//! a buffer already in memory. [`PartProvider`] is that seam.
use crate::{Chunk, ChunkType, archive::ArchiveHeader};
use std::io::{self, Read};

/// Supplies the next physical part of a multipart archive.
pub trait PartProvider<R: Read> {
    /// Opens the part whose AHED archive number is expected to equal `expected`.
    ///
    /// `expected` counts archives from 0, so it is one less than the `.partN.pna`
    /// suffix conventionally given to the same part on disk. The first part is
    /// supplied by the caller, so `expected` is always at least 1.
    ///
    /// Returning `Ok(None)` reports that the required part is unavailable and
    /// fails the current cursor rather than implicitly retrying it.
    ///
    /// # Errors
    ///
    /// Returns any error encountered while locating or opening the part.
    fn next_part(&mut self, expected: u32) -> io::Result<Option<R>>;
}

/// Any closure of the same shape is a provider.
impl<R: Read, F: FnMut(u32) -> io::Result<Option<R>>> PartProvider<R> for F {
    #[inline]
    fn next_part(&mut self, expected: u32) -> io::Result<Option<R>> {
        self(expected)
    }
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

/// Returns the archive number the part after `current` must carry.
///
/// # Errors
///
/// Returns an error if `current` is [`u32::MAX`].
pub(crate) fn next_part_number(current: u32) -> io::Result<u32> {
    current
        .checked_add(1)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "archive number overflow"))
}

/// The error reported when a provider cannot supply a part the archive needs.
pub(crate) fn missing_part_error(expected: u32) -> io::Error {
    io::Error::new(
        io::ErrorKind::NotFound,
        format!("archive part {expected} is required"),
    )
}

/// Validates a part's opening chunk and returns the header it carries.
///
/// # Errors
///
/// Returns an error if `chunk` is not an `AHED`, if its body is malformed, or
/// if it numbers the archive anything other than `expected`.
pub(crate) fn part_header(
    chunk: &(impl Chunk + ?Sized),
    expected: u32,
) -> io::Result<ArchiveHeader> {
    if chunk.ty() != ChunkType::AHED {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected `{}`, got `{}`", ChunkType::AHED, chunk.ty()),
        ));
    }
    let header = ArchiveHeader::try_from_bytes(chunk.data())?;
    if header.archive_number != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "next archive number must be {expected}, got {}",
                header.archive_number
            ),
        ));
    }
    Ok(header)
}
