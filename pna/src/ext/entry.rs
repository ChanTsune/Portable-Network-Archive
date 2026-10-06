//! Provides extension traits for [`NormalEntry`].
use super::private;
use libpna::{
    DirEntryBuilder, FileEntryBuilder, Metadata, NormalEntry, SymlinkEntryBuilder, WriteOptions,
};
use std::{fs, io, path::Path};

/// [`NormalEntry`] filesystem extension methods.
pub trait EntryFsExt: private::Sealed {
    /// Creates an entry from the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be represented as an entry name,
    /// filesystem metadata or contents cannot be read, or entry encoding fails.
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Creates an entry from the given path with options.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be represented as an entry name,
    /// filesystem metadata or contents cannot be read, or entry encoding fails.
    fn from_path_with<P: AsRef<Path>>(path: P, options: WriteOptions) -> io::Result<Self>
    where
        Self: Sized;

    /// Creates an entry from the given path without following symlinks.
    ///
    /// # Errors
    ///
    /// Returns an error if the path or link target cannot be represented in the
    /// archive, filesystem metadata or contents cannot be read, or entry encoding fails.
    fn from_path_symlink<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Creates an entry from the given path with options, without following symlinks.
    ///
    /// `options` applies only to regular file contents.
    ///
    /// # Errors
    ///
    /// Returns an error if the path or link target cannot be represented in the
    /// archive, filesystem metadata or contents cannot be read, or entry encoding fails.
    fn from_path_symlink_with<P: AsRef<Path>>(path: P, options: WriteOptions) -> io::Result<Self>
    where
        Self: Sized;
}

impl EntryFsExt for NormalEntry {
    /// Creates an entry from the given path.
    ///
    /// Reads regular file contents using default [`WriteOptions`]. Other file
    /// types are stored as directory entries. Symbolic links are followed; use
    /// [`EntryFsExt::from_path_symlink`] to store the link itself.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pna::NormalEntry;
    /// use pna::prelude::*;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// NormalEntry::from_path("path/to/file")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be represented as an entry name,
    /// filesystem metadata or contents cannot be read, or entry encoding fails.
    #[inline]
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        Self::from_path_with(path.as_ref(), WriteOptions::builder().build())
    }

    /// Creates an entry from the given path with options.
    ///
    /// Behaves like [`EntryFsExt::from_path`], but uses the provided
    /// [`WriteOptions`] when constructing a file entry.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pna::prelude::*;
    /// use pna::{NormalEntry, WriteOptions};
    ///
    /// # fn main() -> std::io::Result<()> {
    /// NormalEntry::from_path_with("path/to/file", WriteOptions::store())?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be represented as an entry name,
    /// filesystem metadata or contents cannot be read, or entry encoding fails.
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
            let mut builder = FileEntryBuilder::new_with_options(name, options)?;
            io::copy(&mut file, &mut builder)?;
            builder.build()
        } else {
            let builder = DirEntryBuilder::new(name);
            builder.build()
        }
    }

    /// Creates an entry from the given path without following symlinks.
    ///
    /// Stores a symbolic link's target rather than its contents. For other file
    /// types, behaves like [`EntryFsExt::from_path`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pna::NormalEntry;
    /// use pna::prelude::*;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// NormalEntry::from_path_symlink("path/to/file")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the path or link target cannot be represented in the
    /// archive, filesystem metadata or contents cannot be read, or entry encoding fails.
    #[inline]
    fn from_path_symlink<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized,
    {
        Self::from_path_symlink_with(path, WriteOptions::builder().build())
    }

    /// Creates an entry from the given path with options, without following symlinks.
    ///
    /// Like [`EntryFsExt::from_path_symlink`], but applies `options` to regular
    /// file contents. Symbolic-link and directory entries use no compression
    /// or encryption.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pna::prelude::*;
    /// use pna::{NormalEntry, WriteOptions};
    ///
    /// # fn main() -> std::io::Result<()> {
    /// NormalEntry::from_path_symlink_with("path/to/file", WriteOptions::store())?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the path or link target cannot be represented in the
    /// archive, filesystem metadata or contents cannot be read, or entry encoding fails.
    #[inline]
    fn from_path_symlink_with<P: AsRef<Path>>(path: P, options: WriteOptions) -> io::Result<Self>
    where
        Self: Sized,
    {
        let path = path.as_ref();
        let meta = fs::symlink_metadata(path)?;
        let name = path.try_into().map_err(io::Error::other)?;
        if meta.file_type().is_symlink() {
            let target = fs::read_link(path)?;
            let reference = target.as_path().try_into().map_err(io::Error::other)?;
            let mut builder = SymlinkEntryBuilder::new(name, reference)?;
            #[cfg(windows)]
            let ltp = {
                use std::os::windows::fs::FileTypeExt;
                let ft = meta.file_type();
                if ft.is_symlink_dir() {
                    libpna::LinkTargetType::Directory
                } else if ft.is_symlink_file() {
                    libpna::LinkTargetType::File
                } else {
                    libpna::LinkTargetType::Unknown
                }
            };
            #[cfg(not(windows))]
            let ltp = match fs::metadata(path) {
                Ok(m) if m.is_dir() => libpna::LinkTargetType::Directory,
                Ok(m) if m.is_file() => libpna::LinkTargetType::File,
                Ok(_) => libpna::LinkTargetType::Unknown,
                Err(e) if e.kind() == io::ErrorKind::NotFound => libpna::LinkTargetType::Unknown,
                Err(e) => return Err(e),
            };
            builder.metadata(Metadata::new().with_link_target_type(Some(ltp)));
            builder.build()
        } else if meta.is_file() {
            let mut file = fs::File::open(path)?;
            let mut builder = FileEntryBuilder::new_with_options(name, options)?;
            io::copy(&mut file, &mut builder)?;
            builder.build()
        } else {
            let builder = DirEntryBuilder::new(name);
            builder.build()
        }
    }
}
