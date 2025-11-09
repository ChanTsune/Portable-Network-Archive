use crate::util::{str::join_with_capacity, utf8path::normalize_utf8path};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use std::borrow::Cow;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::{self, Utf8Error};

/// A UTF-8 encoded entry name.
///
/// # Examples
///
/// ```rust
/// use libpna::EntryName;
///
/// assert_eq!("uer/bin", EntryName::from("uer/bin"));
/// assert_eq!("user/bin", EntryName::from("/user/bin"));
/// assert_eq!("user/bin", EntryName::from("/user/bin/"));
/// assert_eq!("user/bin", EntryName::from("../user/bin/"));
/// assert_eq!("", EntryName::from("/"));
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryName(String);

impl EntryName {
    fn new_from_utf8path(path: &Utf8Path) -> Self {
        let path = normalize_utf8path(path);
        let iter = path.components().filter_map(|c| match c {
            Utf8Component::Prefix(_)
            | Utf8Component::RootDir
            | Utf8Component::CurDir
            | Utf8Component::ParentDir => None,
            Utf8Component::Normal(p) => Some(p),
        });
        Self(join_with_capacity(iter, "/", path.as_str().len()))
    }

    #[inline]
    fn new_from_utf8(name: &str) -> Self {
        Self::new_from_utf8path(&Utf8PathBuf::from(name))
    }

    #[inline]
    fn new_from_path(name: &Path) -> Result<Self, EntryNameError> {
        let name = str::from_utf8(name.as_os_str().as_encoded_bytes())?;
        Ok(Self::new_from_utf8path(Utf8Path::new(name)))
    }

    /// Creates an [`EntryName`] from a struct impl <code>[Into]<[PathBuf]></code>.
    ///
    /// Any non-Unicode sequences are replaced with
    /// [`U+FFFD REPLACEMENT CHARACTER`][U+FFFD] and
    /// any path components not match with [Component::Normal] are removed.
    ///
    /// [U+FFFD]: core::char::REPLACEMENT_CHARACTER
    /// [Component::Normal]: std::path::Component::Normal
    /// # Examples
    /// ```rust
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

    #[inline]
    fn from_path_lossy(p: &Path) -> Self {
        Self::new_from_utf8(&p.to_string_lossy())
    }

    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Extracts a string slice containing the entire [EntryName].
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryName;
    ///
    /// let name = EntryName::from("foo");
    /// assert_eq!("foo", name.as_str());
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
    /// use libpna::EntryName;
    /// use std::ffi::OsStr;
    ///
    /// let entry_name = EntryName::from("foo.txt");
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
    /// use libpna::EntryName;
    /// use std::path::Path;
    ///
    /// let entry_name = EntryName::from("test/foo.txt");
    /// assert_eq!(Path::new("test/foo.txt"), entry_name.as_path());
    /// ```
    #[inline]
    pub fn as_path(&self) -> &Path {
        self.0.as_ref()
    }
}

impl From<String> for EntryName {
    #[inline]
    fn from(value: String) -> Self {
        Self::new_from_utf8(&value)
    }
}

impl From<&String> for EntryName {
    #[inline]
    fn from(value: &String) -> Self {
        Self::new_from_utf8(value)
    }
}

impl From<&str> for EntryName {
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryName;
    ///
    /// assert_eq!("test.txt", EntryName::from("test.txt"));
    /// assert_eq!("test.txt", EntryName::from("/test.txt"));
    /// assert_eq!("test.txt", EntryName::from("./test.txt"));
    /// assert_eq!("test.txt", EntryName::from("../test.txt"));
    /// ```
    #[inline]
    fn from(value: &str) -> Self {
        Self::new_from_utf8(value)
    }
}

impl From<Cow<'_, str>> for EntryName {
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryName;
    /// use std::borrow::Cow;
    ///
    /// assert_eq!("test.txt", EntryName::from(Cow::from("test.txt")));
    /// ```
    #[inline]
    fn from(value: Cow<'_, str>) -> Self {
        Self::new_from_utf8(&value)
    }
}

impl From<&Cow<'_, str>> for EntryName {
    #[inline]
    fn from(value: &Cow<'_, str>) -> Self {
        Self::new_from_utf8(value)
    }
}

impl TryFrom<&OsStr> for EntryName {
    type Error = EntryNameError;

    #[inline]
    fn try_from(value: &OsStr) -> Result<Self, Self::Error> {
        Self::new_from_path(Path::new(value))
    }
}

impl TryFrom<OsString> for EntryName {
    type Error = EntryNameError;

    #[inline]
    fn try_from(value: OsString) -> Result<Self, Self::Error> {
        Self::new_from_path(Path::new(&value))
    }
}

impl TryFrom<&OsString> for EntryName {
    type Error = EntryNameError;

    #[inline]
    fn try_from(value: &OsString) -> Result<Self, Self::Error> {
        Self::new_from_path(Path::new(value))
    }
}

impl TryFrom<Cow<'_, OsStr>> for EntryName {
    type Error = EntryNameError;

    #[inline]
    fn try_from(value: Cow<'_, OsStr>) -> Result<Self, Self::Error> {
        Self::new_from_path(Path::new(&value))
    }
}

impl TryFrom<&Path> for EntryName {
    type Error = EntryNameError;

    /// # Examples
    ///
    /// ```
    /// use libpna::EntryName;
    /// use std::path::Path;
    ///
    /// let p = Path::new("path/to/file");
    /// assert_eq!("path/to/file", EntryName::try_from(p).unwrap());
    /// ```
    #[inline]
    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        Self::new_from_path(value)
    }
}

impl TryFrom<PathBuf> for EntryName {
    type Error = EntryNameError;

    /// # Examples
    ///
    /// ```
    /// use libpna::EntryName;
    /// use std::path::PathBuf;
    ///
    /// let p = PathBuf::from("path/to/file");
    /// assert_eq!("path/to/file", EntryName::try_from(p).unwrap());
    /// ```
    #[inline]
    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        Self::new_from_path(&value)
    }
}

impl TryFrom<&PathBuf> for EntryName {
    type Error = EntryNameError;

    /// # Examples
    ///
    /// ```
    /// use libpna::EntryName;
    /// use std::path::PathBuf;
    ///
    /// let p = PathBuf::from("path/to/file");
    /// assert_eq!("path/to/file", EntryName::try_from(&p).unwrap());
    /// ```
    #[inline]
    fn try_from(value: &PathBuf) -> Result<Self, Self::Error> {
        Self::new_from_path(value)
    }
}

impl TryFrom<Cow<'_, Path>> for EntryName {
    type Error = EntryNameError;

    /// # Examples
    ///
    /// ```
    /// use libpna::EntryName;
    /// use std::borrow::Cow;
    /// use std::path::PathBuf;
    ///
    /// let p = Cow::from(PathBuf::from("path/to/file"));
    /// assert_eq!("path/to/file", EntryName::try_from(p).unwrap());
    /// ```
    #[inline]
    fn try_from(value: Cow<'_, Path>) -> Result<Self, Self::Error> {
        Self::new_from_path(&value)
    }
}

impl TryFrom<&[u8]> for EntryName {
    type Error = EntryNameError;

    #[inline]
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self::from(str::from_utf8(value)?))
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
        self.as_str()
    }
}

impl AsRef<OsStr> for EntryName {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        self.as_os_str()
    }
}

impl AsRef<Path> for EntryName {
    #[inline]
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl PartialEq<str> for EntryName {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(self.as_str(), other)
    }
}

impl PartialEq<&str> for EntryName {
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryName;
    ///
    /// assert_eq!(EntryName::from("test.txt"), "test.txt");
    /// ```
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
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryName;
    ///
    /// assert_eq!("test.txt", EntryName::from("test.txt"));
    /// ```
    #[inline]
    fn eq(&self, other: &EntryName) -> bool {
        PartialEq::eq(self, &other.as_str())
    }
}

/// Error of invalid [EntryName].
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct EntryNameError(Utf8Error);

impl Error for EntryNameError {}

impl Display for EntryNameError {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl From<Utf8Error> for EntryNameError {
    #[inline]
    fn from(value: Utf8Error) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn remove_root() {
        assert_eq!("test.txt", EntryName::from("/test.txt"));
        assert_eq!("test/test.txt", EntryName::from("/test/test.txt"));
    }

    #[test]
    fn remove_last() {
        assert_eq!("test", EntryName::from("test/"));
        assert_eq!("test/test", EntryName::from("test/test/"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn remove_prefix() {
        assert_eq!("test.txt", EntryName::from("C:\\test.txt"));
        assert_eq!("test/test.txt", EntryName::from("C:\\test\\test.txt"));
    }

    #[test]
    fn basic_string_conversion() {
        // String conversion
        assert_eq!("test.txt", EntryName::from(String::from("test.txt")));
        assert_eq!("test.txt", EntryName::from(&String::from("test.txt")));

        // &str conversion
        assert_eq!("test.txt", EntryName::from("test.txt"));

        // Cow conversion
        assert_eq!("test.txt", EntryName::from(Cow::from("test.txt")));
        assert_eq!("test.txt", EntryName::from(&Cow::from("test.txt")));
    }

    #[test]
    fn special_characters() {
        // Unicode characters
        assert_eq!("日本語.txt", EntryName::from("日本語.txt"));
        assert_eq!("test/日本語.txt", EntryName::from("test/日本語.txt"));
        assert_eq!("日本語/テスト.txt", EntryName::from("日本語/テスト.txt"));

        // Special characters
        assert_eq!("test@example.com", EntryName::from("test@example.com"));
        assert_eq!("test#123", EntryName::from("test#123"));
        assert_eq!("test$123", EntryName::from("test$123"));
        assert_eq!("test+123", EntryName::from("test+123"));
        assert_eq!("test-123", EntryName::from("test-123"));
        assert_eq!("test_123", EntryName::from("test_123"));
    }

    #[test]
    fn path_normalization() {
        // Current directory
        assert_eq!("test.txt", EntryName::from("./test.txt"));
        assert_eq!("test/test.txt", EntryName::from("./test/test.txt"));

        // Parent directory
        assert_eq!("test.txt", EntryName::from("../test.txt"));
        assert_eq!("test/test.txt", EntryName::from("../test/test.txt"));
        assert_eq!("test.txt", EntryName::from("test/../test.txt"));

        // Multiple slashes
        assert_eq!("test/test.txt", EntryName::from("test//test.txt"));
        assert_eq!("test/test.txt", EntryName::from("test///test.txt"));
        assert_eq!("test/test.txt", EntryName::from("///test///test.txt"));
    }

    #[test]
    fn error_cases() {
        // Invalid UTF-8
        let invalid_utf8: &[u8] = &[0x74, 0x65, 0x73, 0x74, 0xFF, 0x2E, 0x74, 0x78, 0x74];
        assert!(EntryName::try_from(invalid_utf8).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn unix_error_cases() {
        let invalid_bytes = [0x74, 0x65, 0x73, 0x74, 0xFF, 0x2E, 0x74, 0x78, 0x74];
        let invalid_os_str = OsStr::from_bytes(&invalid_bytes);
        assert!(EntryName::try_from(invalid_os_str).is_err());
    }

    #[test]
    fn type_conversions() {
        // Path conversions
        let path = Path::new("test.txt");
        assert_eq!("test.txt", EntryName::try_from(path).unwrap());

        let path_buf = PathBuf::from("test.txt");
        assert_eq!("test.txt", EntryName::try_from(&path_buf).unwrap());

        // OsStr conversions
        let os_str = OsStr::new("test.txt");
        assert_eq!("test.txt", EntryName::try_from(os_str).unwrap());

        let os_string = OsString::from("test.txt");
        assert_eq!("test.txt", EntryName::try_from(&os_string).unwrap());
    }

    #[test]
    fn comparisons() {
        let name1 = EntryName::from("test.txt");
        let name2 = EntryName::from("test.txt");
        let name3 = EntryName::from("other.txt");

        // Equality
        assert_eq!(name1, name2);
        assert_eq!(name1, "test.txt");
        assert_eq!("test.txt", name1);

        // Inequality
        assert_ne!(name1, name3);
        assert_ne!(name1, "other.txt");
        assert_ne!("other.txt", name1);
    }

    #[cfg(unix)]
    #[test]
    fn unix_lossy_conversion() {
        // Test with invalid UTF-8 sequence
        let invalid_bytes = [0x74, 0x65, 0x73, 0x74, 0xFF, 0x2E, 0x74, 0x78, 0x74];
        let invalid_path = PathBuf::from(OsStr::from_bytes(&invalid_bytes));
        let name = EntryName::from_lossy(invalid_path);
        assert_eq!("test\u{FFFD}.txt", name.as_str());

        // Test with multiple invalid UTF-8 sequences
        let invalid_bytes = [0x74, 0x65, 0x73, 0x74, 0xFF, 0xFF, 0x2E, 0x74, 0x78, 0x74];
        let invalid_path = PathBuf::from(OsStr::from_bytes(&invalid_bytes));
        let name = EntryName::from_lossy(invalid_path);
        assert_eq!("test\u{FFFD}\u{FFFD}.txt", name.as_str());

        // Test with invalid UTF-8 sequence at the start
        let invalid_bytes = [0xFF, 0x74, 0x65, 0x73, 0x74, 0x2E, 0x74, 0x78, 0x74];
        let invalid_path = PathBuf::from(OsStr::from_bytes(&invalid_bytes));
        let name = EntryName::from_lossy(invalid_path);
        assert_eq!("\u{FFFD}test.txt", name.as_str());

        // Test with invalid UTF-8 sequence at the end
        let invalid_bytes = [0x74, 0x65, 0x73, 0x74, 0x2E, 0x74, 0x78, 0x74, 0xFF];
        let invalid_path = PathBuf::from(OsStr::from_bytes(&invalid_bytes));
        let name = EntryName::from_lossy(invalid_path);
        assert_eq!("test.txt\u{FFFD}", name.as_str());
    }

    #[test]
    fn as_ref_implementations() {
        let name = EntryName::from("test.txt");

        // AsRef<str>
        let str_ref: &str = name.as_ref();
        assert_eq!("test.txt", str_ref);

        // AsRef<OsStr>
        let os_str_ref: &OsStr = name.as_ref();
        assert_eq!(OsStr::new("test.txt"), os_str_ref);

        // AsRef<Path>
        let path_ref: &Path = name.as_ref();
        assert_eq!(Path::new("test.txt"), path_ref);
    }
}
