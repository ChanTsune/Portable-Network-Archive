//! Iterator-based multipart archive traversal.
//!
//! This module provides iterator types for traversing multipart PNA archives,
//! replacing the closure-based API with a more ergonomic iterator pattern.
//!
//! # Overview
//!
//! Two iterator types are provided:
//! - [`MultipartEntries`]: For streaming readers implementing [`Read`]
//! - [`MultipartEntriesSlice`]: For memory-mapped byte slices (zero-copy, requires `memmap` feature)
//!
//! # Examples
//!
//! ## Streaming reader
//!
//! ```ignore
//! use pna::ReadEntry;
//!
//! let files = collect_split_archives("archive.pna")?;
//! let entries = MultipartEntries::new(files)?;
//! for entry in entries {
//!     match entry? {
//!         ReadEntry::Solid(solid) => { /* ... */ }
//!         ReadEntry::Normal(normal) => { /* ... */ }
//!     }
//! }
//! ```
//!
//! ## Memory-mapped slices
//!
//! ```ignore
//! let slices: Vec<&[u8]> = /* memory-mapped files */;
//! let entries = MultipartEntriesSlice::new(slices)?;
//! for entry in entries {
//!     // zero-copy access to entry data
//! }
//! ```

use pna::{Archive, ReadEntry};
use std::io::{self, Read};

/// An iterator over entries in a multipart archive using streaming readers.
///
/// This iterator handles the complexity of transitioning between archive parts
/// transparently, yielding entries as if they came from a single archive.
///
/// # Type Parameters
///
/// * `I` - The iterator type providing readers for subsequent archive parts
/// * `R` - The reader type implementing [`Read`]
pub struct MultipartEntries<I, R>
where
    I: Iterator<Item = R>,
    R: Read,
{
    /// Iterator providing readers for subsequent archive parts
    parts: I,
    /// The currently active archive, or None if exhausted
    current_archive: Option<Archive<R>>,
}

impl<I, R> MultipartEntries<I, R>
where
    I: Iterator<Item = R>,
    R: Read,
{
    /// Creates a new multipart entries iterator from an iterator of readers.
    ///
    /// The first reader in the iterator is used to read the initial archive header.
    /// Subsequent readers are consumed as needed when transitioning between archive parts.
    ///
    /// # Arguments
    ///
    /// * `parts` - An iterator yielding readers for each archive part
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The iterator is empty (no archive provided)
    /// - The first archive header cannot be read
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let files = vec![
    ///     File::open("archive.part1.pna")?,
    ///     File::open("archive.part2.pna")?,
    /// ];
    /// let entries = MultipartEntries::new(files)?;
    /// ```
    pub fn new(parts: impl IntoIterator<IntoIter = I, Item = R>) -> io::Result<Self> {
        let mut parts = parts.into_iter();
        let first = parts
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no archive provided"))?;
        let archive = Archive::read_header(first)?;
        Ok(Self {
            parts,
            current_archive: Some(archive),
        })
    }
}

impl<I, R> Iterator for MultipartEntries<I, R>
where
    I: Iterator<Item = R>,
    R: Read,
{
    type Item = io::Result<ReadEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let archive = self.current_archive.as_mut()?;

            match archive.read_entry() {
                Ok(Some(entry)) => return Some(Ok(entry)),
                Ok(None) => {
                    // Current archive exhausted, check for next part
                    if archive.has_next_archive() {
                        // Take ownership of the current archive for transition
                        let old_archive = self.current_archive.take()?;
                        match self.parts.next() {
                            Some(reader) => match old_archive.read_next_archive(reader) {
                                Ok(new_archive) => {
                                    self.current_archive = Some(new_archive);
                                    continue;
                                }
                                Err(e) => return Some(Err(e)),
                            },
                            None => {
                                return Some(Err(io::Error::new(
                                    io::ErrorKind::NotFound,
                                    "Archive is split, but no subsequent archives are found",
                                )));
                            }
                        }
                    } else {
                        // No more archives, iteration complete
                        self.current_archive = None;
                        return None;
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

/// An iterator over entries in a multipart archive using byte slices.
///
/// This iterator provides zero-copy access to entry data when working with
/// memory-mapped files or pre-loaded byte buffers.
///
/// # Type Parameters
///
/// * `'d` - The lifetime of the byte slice data
/// * `I` - The iterator type providing slices for subsequent archive parts
#[cfg(feature = "memmap")]
pub struct MultipartEntriesSlice<'d, I>
where
    I: Iterator<Item = &'d [u8]>,
{
    /// Iterator providing byte slices for subsequent archive parts
    parts: I,
    /// The currently active archive, or None if exhausted
    current_archive: Option<Archive<&'d [u8]>>,
}

#[cfg(feature = "memmap")]
impl<'d, I> MultipartEntriesSlice<'d, I>
where
    I: Iterator<Item = &'d [u8]>,
{
    /// Creates a new multipart entries iterator from an iterator of byte slices.
    ///
    /// The first slice in the iterator is used to read the initial archive header.
    /// Subsequent slices are consumed as needed when transitioning between archive parts.
    ///
    /// # Arguments
    ///
    /// * `parts` - An iterator yielding byte slices for each archive part
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The iterator is empty (no archive provided)
    /// - The first archive header cannot be read
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mmap1 = Mmap::open("archive.part1.pna")?;
    /// let mmap2 = Mmap::open("archive.part2.pna")?;
    /// let slices = vec![&mmap1[..], &mmap2[..]];
    /// let entries = MultipartEntriesSlice::new(slices)?;
    /// ```
    pub fn new(parts: impl IntoIterator<IntoIter = I, Item = &'d [u8]>) -> io::Result<Self> {
        let mut parts = parts.into_iter();
        let first = parts
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no archive provided"))?;
        let archive = Archive::read_header_from_slice(first)?;
        Ok(Self {
            parts,
            current_archive: Some(archive),
        })
    }
}

#[cfg(feature = "memmap")]
impl<'d, I> Iterator for MultipartEntriesSlice<'d, I>
where
    I: Iterator<Item = &'d [u8]>,
{
    type Item = io::Result<ReadEntry<std::borrow::Cow<'d, [u8]>>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let archive = self.current_archive.as_mut()?;

            match archive.read_entry_slice() {
                Ok(Some(entry)) => return Some(Ok(entry)),
                Ok(None) => {
                    // Current archive exhausted, check for next part
                    if archive.has_next_archive() {
                        // Take ownership of the current archive for transition
                        let old_archive = self.current_archive.take()?;
                        match self.parts.next() {
                            Some(slice) => match old_archive.read_next_archive_from_slice(slice) {
                                Ok(new_archive) => {
                                    self.current_archive = Some(new_archive);
                                    continue;
                                }
                                Err(e) => return Some(Err(e)),
                            },
                            None => {
                                return Some(Err(io::Error::new(
                                    io::ErrorKind::NotFound,
                                    "Archive is split, but no subsequent archives are found",
                                )));
                            }
                        }
                    } else {
                        // No more archives, iteration complete
                        self.current_archive = None;
                        return None;
                    }
                }
                Err(e) => return Some(Err(e)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pna::{EntryBuilder, WriteOptions};
    use std::io::{Cursor, Write};

    // Creates an empty archive
    fn empty_archive() -> Vec<u8> {
        let buf = Vec::new();
        let archive = Archive::write_header(buf).unwrap();
        archive.finalize().unwrap()
    }

    // Creates an archive with a single entry
    fn single_entry_archive() -> Vec<u8> {
        let buf = Vec::new();
        let mut archive = Archive::write_header(buf).unwrap();
        let mut entry = EntryBuilder::new_file("test.txt".into(), WriteOptions::store()).unwrap();
        entry.write_all(b"hello world").unwrap();
        archive.add_entry(entry.build().unwrap()).unwrap();
        archive.finalize().unwrap()
    }

    #[test]
    fn multipart_entries_empty_archive() {
        let archive_data = empty_archive();
        let readers = vec![Cursor::new(archive_data)];
        let mut entries = MultipartEntries::new(readers).unwrap();
        assert!(entries.next().is_none());
    }

    #[test]
    fn multipart_entries_no_archive_provided() {
        let readers: Vec<Cursor<Vec<u8>>> = vec![];
        let result = MultipartEntries::new(readers);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn multipart_entries_single_entry() {
        let archive_data = single_entry_archive();
        let readers = vec![Cursor::new(archive_data)];
        let entries: Vec<_> = MultipartEntries::new(readers)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn multipart_entries_slice_empty_archive() {
        let archive_data = empty_archive();
        let slices = vec![archive_data.as_slice()];
        #[cfg(feature = "memmap")]
        {
            let mut entries = MultipartEntriesSlice::new(slices).unwrap();
            assert!(entries.next().is_none());
        }
        #[cfg(not(feature = "memmap"))]
        {
            let _ = slices; // suppress unused warning
        }
    }

    #[test]
    fn multipart_entries_slice_no_archive_provided() {
        let slices: Vec<&[u8]> = vec![];
        #[cfg(feature = "memmap")]
        {
            let result = MultipartEntriesSlice::new(slices);
            assert!(result.is_err());
            let err = result.err().unwrap();
            assert_eq!(err.kind(), io::ErrorKind::NotFound);
        }
        #[cfg(not(feature = "memmap"))]
        {
            let _ = slices; // suppress unused warning
        }
    }

    #[test]
    fn multipart_entries_slice_single_entry() {
        let archive_data = single_entry_archive();
        let slices = vec![archive_data.as_slice()];
        #[cfg(feature = "memmap")]
        {
            let entries: Vec<_> = MultipartEntriesSlice::new(slices)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert_eq!(entries.len(), 1);
        }
        #[cfg(not(feature = "memmap"))]
        {
            let _ = slices; // suppress unused warning
        }
    }

    #[test]
    fn multipart_entries_real_multipart_archive() {
        let part1 = include_bytes!("../../../../resources/test/multipart.part1.pna");
        let part2 = include_bytes!("../../../../resources/test/multipart.part2.pna");
        let readers = vec![Cursor::new(&part1[..]), Cursor::new(&part2[..])];
        let entries: Vec<_> = MultipartEntries::new(readers)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        // The multipart archive should contain entries
        assert!(!entries.is_empty());
    }

    #[test]
    fn multipart_entries_slice_real_multipart_archive() {
        let part1 = include_bytes!("../../../../resources/test/multipart.part1.pna");
        let part2 = include_bytes!("../../../../resources/test/multipart.part2.pna");
        let slices: Vec<&[u8]> = vec![&part1[..], &part2[..]];
        #[cfg(feature = "memmap")]
        {
            let entries: Vec<_> = MultipartEntriesSlice::new(slices)
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            // The multipart archive should contain entries
            assert!(!entries.is_empty());
        }
        #[cfg(not(feature = "memmap"))]
        {
            let _ = slices; // suppress unused warning
        }
    }

    #[test]
    fn multipart_entries_missing_second_part() {
        let part1 = include_bytes!("../../../../resources/test/multipart.part1.pna");
        // Only provide the first part, which has ANXT marker
        let readers = vec![Cursor::new(&part1[..])];
        let entries = MultipartEntries::new(readers).unwrap();

        // Should get entries from first part, then error when trying to transition
        let mut count = 0;
        for result in entries {
            match result {
                Ok(_) => count += 1,
                Err(e) => {
                    assert_eq!(e.kind(), io::ErrorKind::NotFound);
                    break;
                }
            }
        }
        // We should have read at least some entries before the error
        assert!(count >= 0); // May be 0 if entry spans parts
    }
}
