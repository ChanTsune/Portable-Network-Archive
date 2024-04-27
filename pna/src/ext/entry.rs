use super::private;
use libpna::{EntryBuilder, RegularEntry, WriteOptions};
use std::{fs, io, path::Path};

/// [RegularEntry] extension method trait.
pub trait EntryFsExt: private::Sealed {
    /// Create Entry from a given path.
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;
}

impl EntryFsExt for RegularEntry {
    /// Create Entry from a given path.
    ///
    /// # Examples
    /// ```no_run
    /// # use std::io::{self, prelude::*};
    /// use pna::prelude::*;
    /// use pna::RegularEntry;
    ///
    /// # fn main() -> io::Result<()> {
    /// RegularEntry::from_path("path/to/file")?;
    /// # Ok(())
    /// # }
    /// ```
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        fn inner(path: &Path) -> io::Result<RegularEntry> {
            let meta = fs::metadata(path)?;
            let name = path.try_into().map_err(io::Error::other)?;
            if meta.is_file() {
                let mut file = fs::File::open(path)?;
                let mut builder = EntryBuilder::new_file(name, WriteOptions::builder().build())?;
                io::copy(&mut file, &mut builder)?;
                builder.build()
            } else {
                let builder = EntryBuilder::new_dir(name);
                builder.build()
            }
        }
        inner(path.as_ref())
    }
}
