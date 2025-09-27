//! Provides extension traits for [`Archive<fs::File>`].
use super::private;
use libpna::Archive;
use std::path::Path;
use std::{fs, io};

/// [`Archive`] filesystem extension trait.
pub trait ArchiveFsExt: private::Sealed {
    /// Creates a new archive file at `path` and writes the archive header.
    ///
    /// # Errors
    ///
    /// Returns an error if creating the archive fails.
    fn create<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Opens an existing archive file at `path` and reads the header.
    ///
    /// # Errors
    ///
    /// Returns an error if opening the archive fails.
    fn open<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;
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
}
