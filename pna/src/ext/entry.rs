//! Provides extension traits for [`NormalEntry`].
use super::private;
use libpna::{EntryBuilder, NormalEntry, WriteOptions};
use std::{fs, io, path::Path};

/// Extends [`NormalEntry`] with methods for creating entries directly from the filesystem.
///
/// This trait simplifies the process of creating PNA entries from files, directories,
/// and symbolic links.
pub trait EntryFsExt: private::Sealed {
    /// Creates a new archive entry from a filesystem path.
    ///
    /// This function reads the metadata of the file or directory at the given path and
    /// constructs a corresponding [`NormalEntry`]. For regular files, the content is
    /// also read and included in the entry.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path to create the entry from.
    ///
    /// # Errors
    ///
    /// Returns an `io::Result` containing the new [`NormalEntry`] if successful,
    /// or an `io::Error` if the path cannot be accessed or read.
    fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Creates a new archive entry from a filesystem path with custom write options.
    ///
    /// Similar to [`from_path`], but allows specifying [`WriteOptions`] to control
    /// compression and encryption settings for the entry's content.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path to create the entry from.
    /// * `options` - The [`WriteOptions`] to apply to the entry.
    ///
    /// # Errors
    ///
    /// Returns an `io::Result` containing the new [`NormalEntry`] if successful,
    /// or an `io::Error` if the path cannot be accessed or read.
    fn from_path_with<P: AsRef<Path>>(path: P, options: WriteOptions) -> io::Result<Self>
    where
        Self: Sized;

    /// Creates a new archive entry from a filesystem path, without following symbolic links.
    ///
    /// If the given path is a symbolic link, this function creates a symlink entry in
    /// the archive that points to the same target as the original.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path to create the entry from.
    ///
    /// # Errors
    ///
    /// Returns an `io::Result` containing the new [`NormalEntry`] if successful,
    /// or an `io::Error` if the path cannot be accessed or read.
    fn from_path_symlink<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized;

    /// Creates a new archive entry from a path, with options, without following symbolic links.
    ///
    /// Combines the functionality of [`from_path_symlink`] and [`from_path_with`],
    /// allowing for custom [`WriteOptions`] while also correctly handling symbolic links.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path to create the entry from.
    /// * `options` - The [`WriteOptions`] to apply to the entry.
    ///
    /// # Errors
    ///
    /// Returns an `io::Result` containing the new [`NormalEntry`] if successful,
    /// or an `io::Error` if the path cannot be accessed or read.
    fn from_path_symlink_with<P: AsRef<Path>>(path: P, options: WriteOptions) -> io::Result<Self>
    where
        Self: Sized;
}

impl EntryFsExt for NormalEntry {
    /// Creates an entry from the given path.
    ///
    /// The path may refer to a regular file or a directory. For files, the
    /// file contents are read and embedded into the resulting entry using
    /// default [`WriteOptions`] (equivalent to
    /// `WriteOptions::builder().build()`). For directories, an empty directory
    /// entry is created. Symlinks are followed (uses [`std::fs::metadata`]). If
    /// the intention is to archive a symbolic link itself, use
    /// [`EntryBuilder::new_symlink`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pna::prelude::*;
    /// use pna::NormalEntry;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// NormalEntry::from_path("path/to/file")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while creating the entry.
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
    /// Returns an error if an I/O error occurs while creating the entry.
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

    /// Creates an entry from the given path without following symlinks.
    ///
    /// This behaves like [`EntryFsExt::from_path`], but uses
    /// [`std::fs::symlink_metadata`] to avoid following symbolic links.
    /// When `path` is a symbolic link, a symbolic-link entry is created using
    /// [`EntryBuilder::new_symlink`], with the link target captured via
    /// [`std::fs::read_link`]. For regular files and directories, behavior is
    /// identical to [`EntryFsExt::from_path`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use pna::prelude::*;
    /// use pna::NormalEntry;
    ///
    /// # fn main() -> std::io::Result<()> {
    /// NormalEntry::from_path_symlink("path/to/file")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if an I/O error occurs while creating the entry.
    #[inline]
    fn from_path_symlink<P: AsRef<Path>>(path: P) -> io::Result<Self>
    where
        Self: Sized,
    {
        Self::from_path_symlink_with(path, WriteOptions::builder().build())
    }

    /// Creates an entry from the given path with options, without following symlinks.
    ///
    /// Behaves like [`EntryFsExt::from_path_with`], but uses
    /// [`std::fs::symlink_metadata`] and creates a symbolic-link entry for
    /// symlinks instead of following them.
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
    /// Returns an error if an I/O error occurs while creating the entry.
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
            let builder = EntryBuilder::new_symlink(name, reference)?;
            builder.build()
        } else if meta.is_file() {
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
