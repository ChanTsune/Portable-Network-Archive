use crate::util::str::join_with_capacity;
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use std::borrow::Cow;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt::{self, Display, Formatter};
use std::path::{Component, Path, PathBuf};
use std::str::{self, Utf8Error};

/// A UTF-8 encoded entry reference.
///
/// ## Examples
/// ```
/// use libpna::EntryReference;
///
/// assert_eq!("uer/bin", EntryReference::from("uer/bin"));
/// assert_eq!("/user/bin", EntryReference::from("/user/bin"));
/// assert_eq!("/user/bin", EntryReference::from("/user/bin/"));
/// assert_eq!("../user/bin", EntryReference::from("../user/bin/"));
/// assert_eq!("/", EntryReference::from("/"));
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryReference(String);

impl EntryReference {
    fn new_from_utf8path(path: &Utf8Path) -> Self {
        let has_root = path.has_root();
        let has_prefix = path
            .components()
            .any(|it| matches!(&it, Utf8Component::Prefix(_)));
        let p = path.components().filter_map(|it| match it {
            Utf8Component::Prefix(p) => Some(p.as_str()),
            Utf8Component::RootDir => None,
            Utf8Component::CurDir => Some("."),
            Utf8Component::ParentDir => Some(".."),
            Utf8Component::Normal(n) => Some(n),
        });
        let mut s = join_with_capacity(p, "/", path.as_str().len());
        if !has_prefix && has_root {
            s.insert(0, '/');
        };
        Self(s)
    }

    #[inline]
    fn new_from_utf8(name: &str) -> Self {
        Self::new_from_utf8path(&Utf8PathBuf::from(name))
    }

    #[inline]
    fn new_from_path(path: &Path) -> Result<Self, EntryReferenceError> {
        let path = str::from_utf8(path.as_os_str().as_encoded_bytes())?;
        Ok(Self::new_from_utf8path(Utf8Path::new(path)))
    }

    /// Create an [`EntryReference`] from a struct impl <code>[Into]<[PathBuf]></code>.
    ///
    /// Any non-Unicode sequences are replaced with
    /// [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD].
    ///
    /// [U+FFFD]: core::char::REPLACEMENT_CHARACTER
    ///
    /// # Examples
    /// ```
    /// use libpna::EntryReference;
    ///
    /// assert_eq!("foo.txt", EntryReference::from_lossy("foo.txt"));
    /// assert_eq!("/foo.txt", EntryReference::from_lossy("/foo.txt"));
    /// assert_eq!("./foo.txt", EntryReference::from_lossy("./foo.txt"));
    /// assert_eq!("../foo.txt", EntryReference::from_lossy("../foo.txt"));
    /// ```
    #[inline]
    pub fn from_lossy<T: Into<PathBuf>>(p: T) -> Self {
        Self::from_path_lossy(&p.into())
    }

    #[inline]
    fn from_path_lossy(path: &Path) -> Self {
        let has_root = path.has_root();
        let has_prefix = path
            .components()
            .any(|it| matches!(&it, Component::Prefix(_)));
        let p = path.components().filter_map(|it| match it {
            Component::Prefix(p) => Some(p.as_os_str().to_string_lossy()),
            Component::RootDir => None,
            Component::CurDir => Some(Cow::from(".")),
            Component::ParentDir => Some(Cow::from("..")),
            Component::Normal(n) => Some(n.to_string_lossy()),
        });
        let mut s = join_with_capacity(p, "/", path.as_os_str().len());
        if !has_prefix && has_root {
            s.insert(0, '/');
        };
        Self(s)
    }

    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Extracts a string slice containing the entire [EntryReference].
    ///
    /// ## Examples
    ///
    /// ```
    /// use libpna::EntryReference;
    ///
    /// let r = EntryReference::from("foo");
    /// assert_eq!("foo", r.as_str());
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts to an [`OsStr`] slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryReference;
    /// use std::ffi::OsStr;
    ///
    /// let entry_name = EntryReference::from("foo.txt");
    /// let os_str = OsStr::new("foo.txt");
    /// assert_eq!(entry_name.as_os_str(), os_str);
    /// ```
    #[inline]
    pub fn as_os_str(&self) -> &OsStr {
        self.0.as_ref()
    }

    /// Coerces to a [`Path`] slice.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryReference;
    /// use std::path::Path;
    ///
    /// let entry_name = EntryReference::from("test/foo.txt");
    /// assert_eq!(Path::new("test/foo.txt"), entry_name.as_path());
    /// ```
    #[inline]
    pub fn as_path(&self) -> &Path {
        self.0.as_ref()
    }
}

impl From<String> for EntryReference {
    #[inline]
    fn from(value: String) -> Self {
        Self::new_from_utf8(&value)
    }
}

impl From<&String> for EntryReference {
    #[inline]
    fn from(value: &String) -> Self {
        Self::new_from_utf8(value)
    }
}

impl From<&str> for EntryReference {
    /// ## Examples
    /// ```
    /// use libpna::EntryReference;
    ///
    /// assert_eq!("/path/with/root", EntryReference::from("/path/with/root"));
    /// ```
    #[inline]
    fn from(value: &str) -> Self {
        Self::new_from_utf8(value)
    }
}

impl From<Cow<'_, str>> for EntryReference {
    /// ## Examples
    /// ```
    /// use libpna::EntryReference;
    /// use std::borrow::Cow;
    ///
    /// assert_eq!("test.txt", EntryReference::from(Cow::from("test.txt")));
    /// ```
    #[inline]
    fn from(value: Cow<'_, str>) -> Self {
        Self::new_from_utf8(&value)
    }
}

impl From<&Cow<'_, str>> for EntryReference {
    #[inline]
    fn from(value: &Cow<'_, str>) -> Self {
        Self::new_from_utf8(value)
    }
}

impl TryFrom<&OsStr> for EntryReference {
    type Error = EntryReferenceError;

    #[inline]
    fn try_from(value: &OsStr) -> Result<Self, Self::Error> {
        Self::new_from_path(Path::new(value))
    }
}

impl TryFrom<OsString> for EntryReference {
    type Error = EntryReferenceError;

    #[inline]
    fn try_from(value: OsString) -> Result<Self, Self::Error> {
        Self::new_from_path(Path::new(&value))
    }
}

impl TryFrom<&OsString> for EntryReference {
    type Error = EntryReferenceError;

    #[inline]
    fn try_from(value: &OsString) -> Result<Self, Self::Error> {
        Self::new_from_path(Path::new(value))
    }
}

impl TryFrom<Cow<'_, OsStr>> for EntryReference {
    type Error = EntryReferenceError;

    #[inline]
    fn try_from(value: Cow<'_, OsStr>) -> Result<Self, Self::Error> {
        Self::new_from_path(Path::new(&value))
    }
}

impl TryFrom<&Path> for EntryReference {
    type Error = EntryReferenceError;

    /// ## Examples
    /// ```
    /// use libpna::EntryReference;
    /// use std::path::Path;
    ///
    /// let p = Path::new("path/to/file");
    /// assert_eq!("path/to/file", EntryReference::try_from(p).unwrap());
    /// ```
    #[inline]
    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        Self::new_from_path(value)
    }
}

impl TryFrom<PathBuf> for EntryReference {
    type Error = EntryReferenceError;

    /// ## Examples
    /// ```
    /// use libpna::EntryReference;
    /// use std::path::PathBuf;
    ///
    /// let p = PathBuf::from("path/to/file");
    /// assert_eq!("path/to/file", EntryReference::try_from(p).unwrap());
    /// ```
    #[inline]
    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Self::new_from_path(&value)
    }
}

impl TryFrom<&PathBuf> for EntryReference {
    type Error = EntryReferenceError;

    /// ## Examples
    /// ```
    /// use libpna::EntryReference;
    /// use std::path::PathBuf;
    ///
    /// let p = PathBuf::from("path/to/file");
    /// assert_eq!("path/to/file", EntryReference::try_from(&p).unwrap());
    /// ```
    #[inline]
    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        Self::new_from_path(value)
    }
}

impl TryFrom<Cow<'_, Path>> for EntryReference {
    type Error = EntryReferenceError;

    /// ## Examples
    /// ```
    /// use libpna::EntryReference;
    /// use std::borrow::Cow;
    /// use std::path::PathBuf;
    ///
    /// let p = Cow::from(PathBuf::from("path/to/file"));
    /// assert_eq!("path/to/file", EntryReference::try_from(p).unwrap());
    /// ```
    #[inline]
    fn try_from(value: Cow<'_, Path>) -> Result<Self, Self::Error> {
        Self::new_from_path(&value)
    }
}

impl Display for EntryReference {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl AsRef<str> for EntryReference {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<OsStr> for EntryReference {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl AsRef<Path> for EntryReference {
    #[inline]
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl PartialEq<str> for EntryReference {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(self.as_str(), other)
    }
}

impl PartialEq<&str> for EntryReference {
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryReference;
    ///
    /// assert_eq!(EntryReference::from("test.txt"), "test.txt");
    /// ```
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        PartialEq::eq(self.as_str(), *other)
    }
}

impl PartialEq<EntryReference> for str {
    #[inline]
    fn eq(&self, other: &EntryReference) -> bool {
        PartialEq::eq(self, other.as_str())
    }
}

impl PartialEq<EntryReference> for &str {
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryReference;
    ///
    /// assert_eq!("test.txt", EntryReference::from("test.txt"));
    /// ```
    #[inline]
    fn eq(&self, other: &EntryReference) -> bool {
        PartialEq::eq(self, &other.as_str())
    }
}

/// Error of invalid [EntryReference].
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct EntryReferenceError(Utf8Error);

impl Display for EntryReferenceError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Error for EntryReferenceError {}

impl From<Utf8Error> for EntryReferenceError {
    #[inline]
    fn from(value: Utf8Error) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn keep_root() {
        assert_eq!("/test.txt", EntryReference::from("/test.txt"));
        assert_eq!("/test/test.txt", EntryReference::from("/test/test.txt"));
    }

    #[test]
    fn remove_last() {
        assert_eq!("test", EntryReference::from("test/"));
        assert_eq!("test/test", EntryReference::from("test/test/"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn keep_prefix() {
        assert_eq!("C:/test.txt", EntryReference::from("C:\\test.txt"));
        assert_eq!(
            "C:/test/test.txt",
            EntryReference::from("C:\\test\\test.txt")
        );
    }
}
