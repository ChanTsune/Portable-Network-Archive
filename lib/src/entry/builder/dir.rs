//! Builder for directory entries.
use super::EntryBuilderCore;
use crate::{
    Metadata, NormalEntry,
    chunk::RawChunk,
    entry::{EntryHeader, EntryName},
};
use std::io;

/// A builder for creating a directory [`NormalEntry`].
///
/// Directories carry no data content, so this builder does not implement
/// [`Write`](std::io::Write).
///
/// # Examples
///
/// ```
/// # use std::io;
/// use libpna::DirEntryBuilder;
///
/// # fn main() -> io::Result<()> {
/// let entry = DirEntryBuilder::new("dir/".into()).build()?;
/// # Ok(())
/// # }
/// ```
pub struct DirEntryBuilder {
    core: EntryBuilderCore,
}

impl DirEntryBuilder {
    /// Creates a builder for a directory entry.
    #[inline]
    pub const fn new(name: EntryName) -> Self {
        Self {
            core: EntryBuilderCore::new(EntryHeader::for_dir(name)),
        }
    }

    /// Sets the metadata of the entry, replacing any previously set metadata.
    ///
    /// The raw file size and compressed size recorded in the given metadata
    /// are ignored; [`build()`](Self::build) computes them.
    #[inline]
    pub fn metadata(&mut self, metadata: Metadata) -> &mut Self {
        self.core.metadata(metadata);
        self
    }

    /// Adds extra chunk to the entry.
    #[inline]
    pub fn add_extra_chunk<T: Into<RawChunk>>(&mut self, chunk: T) -> &mut Self {
        self.core.add_extra_chunk(chunk);
        self
    }

    /// Consumes this builder and returns the constructed [`NormalEntry`].
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while building entry into buffer.
    #[inline]
    #[must_use = "building an entry without using it is wasteful"]
    pub fn build(self) -> io::Result<NormalEntry> {
        Ok(self.core.build(Vec::new(), None))
    }
}
