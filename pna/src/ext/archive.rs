//! Provides extension traits for [`Archive<fs::File>`].
use super::private;
use libpna::Archive;
use std::path::Path;
use std::{fs, io};

/// [`Archive`] filesystem extension trait.
pub trait ArchiveFsExt: private::Sealed {
    /// Creates a new archive file at `path` and writes the archive header.
    ///
    /// An existing file at `path` is truncated.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or the archive header cannot be written.
    fn create<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Opens an existing archive file at `path` and reads the header.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or the archive header is invalid
    /// or cannot be read.
    fn open<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Opens an existing archive for appending entries.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened for reading and writing,
    /// the archive header is invalid, or the end-of-archive marker cannot be found.
    fn open_for_append<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Opens all parts of a split archive, leaving the last part ready for appending entries.
    ///
    /// `next_part_path` receives the original `path` and the next one-based part
    /// index, starting at 2.
    ///
    /// # Errors
    ///
    /// Returns an error if a part cannot be opened for reading and writing,
    /// an archive header is invalid, the part numbers are not consecutive,
    /// or an end-of-archive marker cannot be found.
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
    /// An existing file at `path` is truncated.
    ///
    /// Equivalent to calling [`Archive::write_header`] with a newly
    /// created [`fs::File`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::io;
    /// use pna::Archive;
    /// use pna::prelude::*;
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
    /// # Examples
    ///
    /// ```no_run
    /// # use std::io;
    /// use pna::Archive;
    /// use pna::prelude::*;
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
    /// Opens the file for reading and writing and positions it before the
    /// end-of-archive marker.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::io;
    /// use pna::Archive;
    /// use pna::prelude::*;
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
    /// Like [`ArchiveFsExt::open_for_append`], but follows subsequent archive parts.
    /// `next_part_path` receives the original `path` and the next one-based part
    /// index, starting at 2. The returned archive is positioned before the last
    /// part's end-of-archive marker.
    ///
    /// # Examples
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
