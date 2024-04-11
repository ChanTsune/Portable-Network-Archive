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
    fn new_from_path(name: &Path) -> Result<Self, EntryNameError> {
        let buf = filtered_components(name)
            .map(|i| try_to_string(i).map_err(|e| EntryNameError(e.to_string())))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self(buf.join("/")))
    }

    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }

    /// Extracts a string slice containing the entire `EntryName`.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryName;
    ///
    /// let name = EntryName::from_lossy("foo");
    ///
    /// assert_eq!("foo", name.as_str());
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }

    /// Converts to an [`OsStr`] slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryName;
    /// use std::ffi::OsStr;
    ///
    /// let entry_name = EntryName::from_lossy("foo.txt");
    /// let os_str = OsStr::new("foo.txt");
    /// assert_eq!(entry_name.as_os_str(), os_str);
    /// ```
    #[inline]
    pub fn as_os_str(&self) -> &OsStr {
        self.as_ref()
    }

    /// Coerces to a [`Path`] slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryName;
    /// use std::path::Path;
    ///
    /// let entry_name = EntryName::from_lossy("test/foo.txt");
    /// assert_eq!(Path::new("test/foo.txt"), entry_name.as_path());
    /// ```
    #[inline]
    pub fn as_path(&self) -> &Path {
        self.as_ref()
    }

    fn from_path_lossy(p: &Path) -> Self {
        let buf = filtered_components(p)
            .map(|i| i.to_string_lossy())
            .collect::<Vec<_>>();
        Self(buf.join("/"))
    }

    /// Create an [`EntryName`] from a struct impl <code>[Into]<[PathBuf]></code>.
    ///
    /// Any non-Unicode sequences are replaced with
    /// [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD] and
    /// any path components not match with [Component::Normal] are removed.
    ///
    /// [U+FFFD]: core::char::REPLACEMENT_CHARACTER
    ///
    /// # Examples
    /// ```
    /// use libpna::EntryName;
    ///
    /// assert_eq!("foo.txt", EntryName::from_lossy("foo.txt"));
    /// assert_eq!("foo.txt", EntryName::from_lossy("/foo.txt"));
    /// assert_eq!("foo.txt", EntryName::from_lossy("./foo.txt"));
    /// assert_eq!("foo.txt", EntryName::from_lossy("../foo.txt"));
    /// ```
    #[inline]
    pub fn from_lossy<T: Into<PathBuf>>(p: T) -> Self {
        Self::from_path_lossy(&p.into())
    }
}

impl TryFrom<&str> for EntryName {
    type Error = EntryNameError;
    /// # Examples
    /// ```
    /// use libpna::EntryName;
    ///
    /// assert_eq!(
    ///     EntryName::try_from("test.txt"),
    ///     EntryName::try_from("test.txt")
    /// );
    ///
    /// assert_eq!(
    ///     EntryName::try_from("/test.txt"),
    ///     EntryName::try_from("test.txt")
    /// );
    ///
    /// assert_eq!(
    ///     EntryName::try_from("./test.txt"),
    ///     EntryName::try_from("test.txt")
    /// );
    ///
    /// assert_eq!(
    ///     EntryName::try_from("../test.txt"),
    ///     EntryName::try_from("test.txt")
    /// );
    /// ```
    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new_from_path(value.as_ref())
    }
}

impl TryFrom<&Path> for EntryName {
    type Error = EntryNameError;

    #[inline]
    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        Self::new_from_path(value)
    }
}

impl TryFrom<&[u8]> for EntryName {
    type Error = EntryNameError;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::try_from(str::from_utf8(value).map_err(|e| EntryNameError(e.to_string()))?)
    }
}

impl Display for EntryName {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
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

impl PartialEq<str> for EntryName {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(self.as_str(), other)
    }
}

impl PartialEq<&str> for EntryName {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        PartialEq::eq(self.as_str(), *other)
    }
}

impl PartialEq<EntryName> for str {
    #[inline]
    fn eq(&self, other: &EntryName) -> bool {
        PartialEq::eq(self, other.as_str())
    }
}

impl PartialEq<EntryName> for &str {
    #[inline]
    fn eq(&self, other: &EntryName) -> bool {
        PartialEq::eq(self, &other.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_root() {
        assert_eq!(
            EntryName::try_from("/test.txt"),
            EntryName::try_from("test.txt")
        );
        assert_eq!(
            EntryName::try_from("/test/test.txt"),
            EntryName::try_from("test/test.txt")
        );
    }

    #[test]
    fn remove_last() {
        assert_eq!(EntryName::try_from("test/"), EntryName::try_from("test"));
        assert_eq!(
            EntryName::try_from("test/test/"),
            EntryName::try_from("test/test")
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn remove_prefix() {
        assert_eq!(
            EntryName::try_from("C:\\test.txt"),
            EntryName::try_from("test.txt")
        );
        assert_eq!(
            EntryName::try_from("C:\\test\\test.txt"),
            EntryName::try_from("test/test.txt")
        );
    }
}
