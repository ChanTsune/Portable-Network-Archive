use super::private;
use libpna::{EntryBuilder, NormalEntry, WriteOptions};
use std::{fs, io, path::Path};

/// [`NormalEntry`] filesystem extension methods.
pub trait EntryFsExt: private::Sealed {
    /// Creates an entry from the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while creating the entry.
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;
    /// Creates an entry from the given path with options.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while creating the entry.
    fn from_path_with<P: AsRef<Path>>(path: P, options: WriteOptions) -> io::Result<Self>
    where
        Self: Sized;
}

impl EntryFsExt for NormalEntry {
    /// Creates an entry from the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while creating the entry.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::io::{self, prelude::*};
    /// use pna::prelude::*;
    /// use pna::NormalEntry;
    ///
    /// # fn main() -> io::Result<()> {
    /// NormalEntry::from_path("path/to/file")?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::from_path_with(path.as_ref(), WriteOptions::builder().build())
    }

    /// Creates an entry from the given path with options.
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while creating the entry.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use std::io::{self, prelude::*};
    /// use pna::prelude::*;
    /// use pna::{NormalEntry, WriteOptions};
    ///
    /// # fn main() -> io::Result<()> {
    /// NormalEntry::from_path_with("path/to/file", WriteOptions::store())?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn from_path_with<P: AsRef<Path>>(path: P, options: WriteOptions) -> io::Result<Self>
    where
        Self: Sized,
    {
        let path = path.as_ref();
        let meta = fs::metadata(path)?;
        let name = path.try_into().map_err(io::Error::other)?;
        if meta.is_file() {
            let mut file = fs::File::open(path)?;
            let mut builder = EntryBuilder::new_file(name, options)?;
            io::copy(&mut file, &mut builder)?;
            builder.build()
        } else {
            let builder = EntryBuilder::new_dir(name);
            builder.build()
        }
    }
}
