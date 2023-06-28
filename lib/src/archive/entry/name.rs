use crate::util::try_to_string;
use std::error::Error;
use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::path::{Component, Path, PathBuf};
use std::str;

/// A UTF-8 encoded entry name.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryName(String);

/// Error of invalid EntryName
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryNameError(String);

impl Error for EntryNameError {}

impl Display for EntryNameError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

fn filtered_components(path: &Path) -> impl Iterator<Item = &OsStr> {
    path.components().filter_map(|c| match c {
        Component::Prefix(_) | Component::RootDir | Component::CurDir | Component::ParentDir => {
            None
        }
        Component::Normal(p) => Some(p),
    })
}

impl EntryName {
    fn new(name: &Path) -> Result<Self, EntryNameError> {
        let buf = filtered_components(name)
            .map(|i| try_to_string(i).map_err(|e| EntryNameError(e.to_string())))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(buf.join("/")))
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }

    #[inline]
    pub fn as_os_str(&self) -> &OsStr {
        self.as_ref()
    }

    #[inline]
    pub fn as_path(&self) -> &Path {
        self.as_ref()
    }

    pub fn from_path_lossy(p: &Path) -> Self {
        let buf = filtered_components(p)
            .map(|i| i.to_string_lossy())
            .collect::<Vec<_>>();
        Self(buf.join("/"))
    }
}

impl<T: Into<PathBuf>> From<T> for EntryName {
    /// # Examples
    /// ```
    /// use libpna::EntryName;
    ///
    /// assert_eq!(EntryName::from("test.txt"), EntryName::from("test.txt"));
    ///
    /// assert_eq!(EntryName::from("/test.txt"), EntryName::from("test.txt"));
    ///
    /// assert_eq!(EntryName::from("./test.txt"), EntryName::from("test.txt"));
    ///
    /// assert_eq!(EntryName::from("../test.txt"), EntryName::from("test.txt"));
    /// ```
    #[inline]
    fn from(value: T) -> Self {
        let value = value.into();
        let buf = value
            .components()
            .filter_map(|c| match c {
                Component::Prefix(_)
                | Component::RootDir
                | Component::CurDir
                | Component::ParentDir => None,
                Component::Normal(p) => Some(p),
            })
            .map(|i| i.to_string_lossy())
            .collect::<Vec<_>>();
        Self(buf.join("/"))
    }
}

impl Display for EntryName {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for EntryName {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<OsStr> for EntryName {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}

impl AsRef<Path> for EntryName {
    #[inline]
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_root() {
        assert_eq!(EntryName::from("/test.txt"), EntryName::from("test.txt"));
        assert_eq!(
            EntryName::from("/test/test.txt"),
            EntryName::from("test/test.txt")
        );
    }

    #[test]
    fn remove_last() {
        assert_eq!(EntryName::from("test/"), EntryName::from("test"));
        assert_eq!(EntryName::from("test/test/"), EntryName::from("test/test"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn remove_prefix() {
        assert_eq!(EntryName::from("C:\\test.txt"), EntryName::from("test.txt"));
        assert_eq!(
            EntryName::from("C:\\test\\test.txt"),
            EntryName::from("test/test.txt")
        );
    }
}
