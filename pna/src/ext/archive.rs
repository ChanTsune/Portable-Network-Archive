use super::private;
use libpna::Archive;
use std::path::Path;
use std::{fs, io};

/// [`Archive`] filesystem extension trait.
pub trait ArchiveFsExt: private::Sealed {
    /// Creates a PNA file.
    ///
    /// # Errors
    ///
    /// Returns an error if creating the archive fails.
    fn create<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Opens an existing PNA file.
    ///
    /// # Errors
    ///
    /// Returns an error if opening the archive fails.
    fn open<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;
}

impl ArchiveFsExt for Archive<fs::File> {
    /// # Examples
    /// ```no_run
    /// # use std::io::{self, prelude::*};
    /// use pna::prelude::*;
    /// use pna::Archive;
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut archive = Archive::create("archive.pna")?;
    /// archive.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn create<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = fs::File::create(path)?;
        Archive::write_header(file)
    }

    /// # Examples
    /// ```no_run
    /// # use std::io::{self, prelude::*};
    /// use pna::prelude::*;
    /// use pna::Archive;
    ///
    /// # fn main() -> io::Result<()> {
    /// let mut archive = Archive::open("archive.pna")?;
    /// archive.finalize()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = fs::File::open(path)?;
        Archive::read_header(file)
    }
}
