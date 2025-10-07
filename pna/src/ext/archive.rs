//! Provides extension traits for [`Archive<fs::File>`].
use super::private;
use libpna::Archive;
use std::path::Path;
use std::{fs, io};

/// Extends the [`Archive`] type with methods for interacting with the filesystem.
///
/// This trait provides convenient shortcuts for creating, opening, and appending to
/// PNA archives that are backed by [`fs::File`].
pub trait ArchiveFsExt: private::Sealed {
    /// Creates a new archive file at the specified `path` and writes the archive header.
    ///
    /// This is a convenience method that combines creating a file and initializing it as a
    /// PNA archive.
    ///
    /// # Arguments
    ///
    /// * `path` - The path where the new archive file will be created.
    ///
    /// # Errors
    ///
    /// Returns an `io::Result` containing the new [`Archive`] instance if successful,
    /// or an `io::Error` if the file cannot be created or the header cannot be written.
    fn create<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Opens an existing archive file at the specified `path` and reads its header.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the existing archive file.
    ///
    /// # Errors
    ///
    /// Returns an `io::Result` containing the [`Archive`] instance if successful,
    /// or an `io::Error` if the file cannot be opened or the header is invalid.
    fn open<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Opens an existing archive file for appending new entries.
    ///
    /// The file is opened with read and write access, and the archive is positioned
    /// at the end, ready for new entries to be added.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the archive file to be opened for appending.
    ///
    /// # Errors
    ///
    /// Returns an `io::Result` containing the [`Archive`] instance if successful,
    /// or an `io::Error` if the file cannot be opened or the archive is corrupt.
    fn open_for_append<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Opens all parts of a multipart archive, preparing the last part for appending.
    ///
    /// This function iterates through a series of archive parts, as determined by the
    /// `next_part_path` closure, and opens them sequentially. The final part is left
    /// open and ready for new entries to be appended.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the first part of the archive.
    /// * `next_part_path` - A closure that, given the path of the current part and its
    ///   index, returns the path to the next part.
    ///
    /// # Errors
    ///
    /// Returns an `io::Result` containing the [`Archive`] instance if successful,
    /// or an `io::Error` if any part of the archive cannot be opened or is invalid.
    fn open_multipart_for_append<P, F, N>(path: P, next_part_path: F) -> io::Result<Self>
    where
        Self: Sized,
        P: AsRef<Path>,
        F: FnMut(&Path, usize) -> N,
        N: AsRef<Path>;
}

impl ArchiveFsExt for Archive<fs::File> {
    /// Creates a new archive file at `path` and writes the archive header.
    ///
    /// Equivalent to calling [`Archive::write_header`] with a newly
    /// created [`fs::File`].
    ///
    /// Returns an `Archive<fs::File>` ready for writing entries.
    ///
    /// # Examples
    /// ```no_run
    /// # use std::io;
    /// use pna::prelude::*;
    /// use pna::Archive;
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut archive = Archive::create("archive.pna")?;
    /// archive.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns any error from [`fs::File::create`] or any error from [`Archive::write_header`].
    #[inline]
    fn create<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = fs::File::create(path)?;
        Archive::write_header(file)
    }

    /// Opens an existing archive file at `path` and reads the header.
    ///
    /// Equivalent to calling [`Archive::read_header`] with a file
    /// opened via [`fs::File::open`].
    ///
    /// Returns an `Archive<fs::File>` ready for reading entries.
    ///
    /// # Examples
    /// ```no_run
    /// # use std::io;
    /// use pna::prelude::*;
    /// use pna::Archive;
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut archive = Archive::open("archive.pna")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns any error from [`fs::File::open`] or any error from [`Archive::read_header`].
    #[inline]
    fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = fs::File::open(path)?;
        Archive::read_header(file)
    }

    /// Opens an existing archive for appending entries.
    ///
    /// This opens the file with read/write permissions, reads the archive
    /// header, and seeks to the end-of-archive marker using
    /// [`Archive::seek_to_end`], so that new entries can be appended safely.
    ///
    /// # Examples
    /// ```no_run
    /// # use std::io;
    /// use pna::prelude::*;
    /// use pna::Archive;
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut archive = Archive::open_for_append("archive.pna")?;
    /// // archive.add_entry(...)?;
    /// archive.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns any error from [`fs::OpenOptions::open`], [`Archive::read_header`]
    /// or [`Archive::seek_to_end`].
    #[inline]
    fn open_for_append<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = fs::OpenOptions::new().read(true).write(true).open(path)?;
        let mut archive = Archive::read_header(file)?;
        archive.seek_to_end()?;
        Ok(archive)
    }

    /// Opens all parts of a split archive, leaving the last part ready for appending entries.
    ///
    /// This behaves like [`ArchiveFsExt::open_for_append`] but is aware of multipart archives.
    /// The first part is opened with read/write access and rewound to just before the end marker.
    /// While an `ANXT` chunk is present, the provided `next_part_path` closure is invoked with the
    /// original first-part path and the next one-based part index to resolve the subsequent file
    /// to open. Each part is read, validated, and rewound in turn so that the final [`Archive`]
    /// returned is positioned to accept new entries safely.
    ///
    /// ```no_run
    /// use pna::Archive;
    /// use pna::prelude::ArchiveFsExt;
    /// use std::io;
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut archive = Archive::open_multipart_for_append("example.part1.pna", |base, index| {
    ///     let mut next = base.to_path_buf();
    ///     next.set_file_name(format!("example.part{index}.pna"));
    ///     next
    /// })?;
    /// // archive.add_entry(...)?;
    /// archive.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns any error from [`fs::OpenOptions::open`], [`Archive::read_header`],
    /// [`Archive::seek_to_end`], or [`Archive::read_next_archive`].
    #[inline]
    fn open_multipart_for_append<P, F, N>(path: P, mut next_part_path: F) -> io::Result<Self>
    where
        P: AsRef<Path>,
        F: FnMut(&Path, usize) -> N,
        N: AsRef<Path>,
    {
        let base = path.as_ref();
        let mut part_index = 1;
        let mut archive = Self::open_for_append(base)?;

        while archive.has_next_archive() {
            part_index += 1;
            let next_path = next_part_path(base, part_index);
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(next_path.as_ref())?;
            archive = archive.read_next_archive(file)?;
            archive.seek_to_end()?;
        }
        Ok(archive)
    }
}
