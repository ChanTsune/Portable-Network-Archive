use super::private;
use libpna::Archive;
use std::path::Path;
use std::{fs, io};

/// [Archive] fs extension trait.
pub trait ArchiveFsExt: private::Sealed {
    /// Create PNA file.
    fn create<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;
    /// Open existing PNA file.
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
    fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = fs::File::open(path)?;
        Archive::read_header(file)
    }
}
