use crate::util::str::join_with_capacity;
use camino::{Utf8Component, Utf8Path};
use std::borrow::Cow;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::{self, Utf8Error};

/// A UTF-8 encoded entry reference.
///
/// # Examples
///
/// ```
/// use libpna::EntryReference;
///
/// assert_eq!("uer/bin", EntryReference::from("uer/bin"));
/// assert_eq!("user/bin", EntryReference::from("/user/bin"));
/// assert_eq!("user/bin", EntryReference::from("/user/bin/"));
/// assert_eq!("../user/bin", EntryReference::from("../user/bin/"));
/// assert_eq!("", EntryReference::from("/"));
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryReference(String);

impl EntryReference {
    #[inline]
    fn new_from_utf8(name: &str) -> Self {
        Self::from_utf8_preserve_root(name).sanitize()
    }

    #[inline]
    fn new_from_path(path: &Path) -> Result<Self, EntryReferenceError> {
        let path = str::from_utf8(path.as_os_str().as_encoded_bytes())?;
        Ok(Self::new_from_utf8(path))
    }

    /// Creates an [`EntryReference`] from a struct impl <code>[Into]<[PathBuf]></code>.
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
    /// assert_eq!("foo.txt", EntryReference::from_lossy("/foo.txt"));
    /// assert_eq!("./foo.txt", EntryReference::from_lossy("./foo.txt"));
    /// assert_eq!("../foo.txt", EntryReference::from_lossy("../foo.txt"));
    /// ```
    #[inline]
    pub fn from_lossy<T: Into<PathBuf>>(p: T) -> Self {
        Self::from_path_lossy(&p.into())
    }

    #[inline]
    fn from_path_lossy(path: &Path) -> Self {
        Self::new_from_utf8(&path.to_string_lossy())
    }

    /// Creates an [EntryReference] from a UTF-8 string while preserving absolute
    /// roots, prefixes, and parent components.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryReference;
    ///
    /// assert_eq!("/foo.txt", EntryReference::from_utf8_preserve_root("/foo.txt"));
    /// assert_eq!("bar/../foo.txt", EntryReference::from_utf8_preserve_root("bar/../foo.txt"));
    /// assert_eq!("../foo.txt", EntryReference::from_utf8_preserve_root("../foo.txt"));
    /// ```
    #[inline]
    pub fn from_utf8_preserve_root(path: &str) -> Self {
        Self::new_preserve_root(path.into())
    }

    #[inline]
    fn new_preserve_root(path: String) -> Self {
        Self(path)
    }

    /// Creates an [EntryReference] from a path, preserving absolute path components.
    ///
    /// # Errors
    ///
    /// Returns an [`EntryReferenceError`] if the path cannot be represented as valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryReference;
    ///
    /// assert_eq!("/foo.txt", EntryReference::from_path_preserve_root("/foo.txt".as_ref()).unwrap());
    /// assert_eq!("../foo.txt", EntryReference::from_path_preserve_root("../foo.txt".as_ref()).unwrap());
    /// ```
    #[inline]
    pub fn from_path_preserve_root(path: &Path) -> Result<Self, EntryReferenceError> {
        let path = str::from_utf8(path.as_os_str().as_encoded_bytes())?;
        Ok(Self::from_utf8_preserve_root(path))
    }

    /// Creates an [EntryReference] from a path, preserving absolute path components.
    ///
    /// Any invalid UTF-8 sequences are replaced.
    ///
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryReference;
    ///
    /// assert_eq!("/foo.txt", EntryReference::from_path_lossy_preserve_root("/foo.txt".as_ref()));
    /// ```
    #[inline]
    pub fn from_path_lossy_preserve_root(path: &Path) -> Self {
        Self::new_preserve_root(path.to_string_lossy().into())
    }

    /// Returns a sanitized reference with root separators removed.
    ///
    /// Unlike [`EntryName::sanitize`](super::EntryName), this preserves prefixes, `.` and `..`
    /// components because hardlink targets may legitimately contain relative
    /// traversals.
    #[inline]
    pub fn sanitize(&self) -> Self {
        let path = Utf8Path::new(&self.0);
        let p = path.components().filter_map(|it| match it {
            Utf8Component::Prefix(p) => Some(p.as_str()),
            Utf8Component::RootDir => None,
            Utf8Component::CurDir => Some("."),
            Utf8Component::ParentDir => Some(".."),
            Utf8Component::Normal(n) => Some(n),
        });
        Self(join_with_capacity(p, "/", path.as_str().len()))
    }

    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Extracts a string slice containing the entire [EntryReference].
    ///
    /// # Examples
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
    /// # Examples
    ///
    /// ```
    /// use libpna::EntryReference;
    ///
    /// assert_eq!("path/with/root", EntryReference::from("/path/with/root"));
    /// ```
    #[inline]
    fn from(value: &str) -> Self {
        Self::new_from_utf8(value)
    }
}

impl From<Cow<'_, str>> for EntryReference {
    /// # Examples
    ///
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

    /// # Examples
    ///
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

    /// # Examples
    ///
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

    /// # Examples
    ///
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

    /// # Examples
    ///
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
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn strip_root() {
        assert_eq!("test.txt", EntryReference::from("/test.txt"));
        assert_eq!("test/test.txt", EntryReference::from("/test/test.txt"));
        assert_eq!("", EntryReference::from("/"));
        assert_eq!("", EntryReference::from("///"));
    }

    #[test]
    fn preserve_root_variants() {
        assert_eq!(
            "/abs/path",
            EntryReference::from_utf8_preserve_root("/abs/path")
        );
        assert_eq!(
            "../rel/path",
            EntryReference::from_utf8_preserve_root("../rel/path")
        );
        // preserve_root stores the string as-is, no separator conversion
        assert_eq!(
            "C:\\drive\\path",
            EntryReference::from_utf8_preserve_root("C:\\drive\\path")
        );
    }

    #[test]
    fn remove_last() {
        assert_eq!("test", EntryReference::from("test/"));
        assert_eq!("test/test", EntryReference::from("test/test/"));
        assert_eq!("test", EntryReference::from("test///"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn keep_prefix() {
        assert_eq!("C:/test.txt", EntryReference::from("C:\\test.txt"));
        assert_eq!(
            "C:/test/test.txt",
            EntryReference::from("C:\\test\\test.txt")
        );
        assert_eq!("C:", EntryReference::from("C:\\"));
        assert_eq!("C:", EntryReference::from("C:\\\\\\"));
    }

    #[test]
    fn basic_string_conversion() {
        // String conversion
        assert_eq!("test.txt", EntryReference::from(String::from("test.txt")));
        assert_eq!("test.txt", EntryReference::from(&String::from("test.txt")));

        // &str conversion
        assert_eq!("test.txt", EntryReference::from("test.txt"));

        // Cow conversion
        assert_eq!("test.txt", EntryReference::from(Cow::from("test.txt")));
        assert_eq!("test.txt", EntryReference::from(&Cow::from("test.txt")));
    }

    #[test]
    fn special_characters() {
        // Unicode characters
        assert_eq!("日本語.txt", EntryReference::from("日本語.txt"));
        assert_eq!("test/日本語.txt", EntryReference::from("test/日本語.txt"));
        assert_eq!(
            "日本語/テスト.txt",
            EntryReference::from("日本語/テスト.txt")
        );

        // Special characters
        assert_eq!("test@example.com", EntryReference::from("test@example.com"));
        assert_eq!("test#123", EntryReference::from("test#123"));
        assert_eq!("test$123", EntryReference::from("test$123"));
        assert_eq!("test+123", EntryReference::from("test+123"));
        assert_eq!("test-123", EntryReference::from("test-123"));
        assert_eq!("test_123", EntryReference::from("test_123"));
    }

    #[test]
    fn path_normalization() {
        // Current directory
        assert_eq!("./test.txt", EntryReference::from("./test.txt"));
        assert_eq!("./test/test.txt", EntryReference::from("./test/test.txt"));
        assert_eq!("./test/test.txt", EntryReference::from("./test/./test.txt"));

        // Parent directory
        assert_eq!("../test.txt", EntryReference::from("../test.txt"));
        assert_eq!("../test/test.txt", EntryReference::from("../test/test.txt"));
        assert_eq!(
            "../test/../test.txt",
            EntryReference::from("../test/../test.txt")
        );

        // Multiple slashes
        assert_eq!("test/test.txt", EntryReference::from("test//test.txt"));
        assert_eq!("test/test.txt", EntryReference::from("test///test.txt"));
        assert_eq!("test/test.txt", EntryReference::from("///test///test.txt"));
    }

    #[test]
    fn preserve_root_edge_cases() {
        // Empty string
        assert_eq!("", EntryReference::from_utf8_preserve_root(""));
        // Only parent dir
        assert_eq!("..", EntryReference::from_utf8_preserve_root(".."));
        // Only current dir
        assert_eq!(".", EntryReference::from_utf8_preserve_root("."));
        // Only root
        assert_eq!("/", EntryReference::from_utf8_preserve_root("/"));
        // Multiple parent dirs
        assert_eq!(
            "../../..",
            EntryReference::from_utf8_preserve_root("../../..")
        );
    }

    #[test]
    fn sanitize_edge_cases() {
        // Empty string remains empty
        assert_eq!("", EntryReference::from_utf8_preserve_root("").sanitize());
        // Only parent dir
        assert_eq!(
            "..",
            EntryReference::from_utf8_preserve_root("..").sanitize()
        );
        // Only current dir
        assert_eq!(".", EntryReference::from_utf8_preserve_root(".").sanitize());
        // Only root
        assert_eq!("", EntryReference::from_utf8_preserve_root("/").sanitize());
        // Multiple parent dirs
        assert_eq!(
            "../../..",
            EntryReference::from_utf8_preserve_root("../../..").sanitize()
        );
        // Mixed with normal component
        assert_eq!(
            "../foo",
            EntryReference::from_utf8_preserve_root("/../foo").sanitize()
        );
        assert_eq!(
            "./foo",
            EntryReference::from_utf8_preserve_root("./foo").sanitize()
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_error_cases() {
        let invalid_bytes = [0x74, 0x65, 0x73, 0x74, 0xFF, 0x2E, 0x74, 0x78, 0x74];
        let invalid_os_str = OsStr::from_bytes(&invalid_bytes);
        assert!(EntryReference::try_from(invalid_os_str).is_err());
    }

    #[test]
    fn type_conversions() {
        // Path conversions
        let path = Path::new("test.txt");
        assert_eq!("test.txt", EntryReference::try_from(path).unwrap());

        let path_buf = PathBuf::from("test.txt");
        assert_eq!("test.txt", EntryReference::try_from(&path_buf).unwrap());

        // OsStr conversions
        let os_str = OsStr::new("test.txt");
        assert_eq!("test.txt", EntryReference::try_from(os_str).unwrap());

        let os_string = OsString::from("test.txt");
        assert_eq!("test.txt", EntryReference::try_from(&os_string).unwrap());
    }

    #[test]
    fn comparisons() {
        let ref1 = EntryReference::from("test.txt");
        let ref2 = EntryReference::from("test.txt");
        let ref3 = EntryReference::from("other.txt");

        // Equality
        assert_eq!(ref1, ref2);
        assert_eq!(ref1, "test.txt");
        assert_eq!("test.txt", ref1);

        // Inequality
        assert_ne!(ref1, ref3);
        assert_ne!(ref1, "other.txt");
        assert_ne!("other.txt", ref1);
    }

    #[cfg(unix)]
    #[test]
    fn unix_lossy_conversion() {
        // Test with invalid UTF-8 sequence
        let invalid_bytes = [0x74, 0x65, 0x73, 0x74, 0xFF, 0x2E, 0x74, 0x78, 0x74];
        let invalid_path = PathBuf::from(OsStr::from_bytes(&invalid_bytes));
        let name = EntryReference::from_lossy(invalid_path);
        assert_eq!("test\u{FFFD}.txt", name.as_str());

        // Test with multiple invalid UTF-8 sequences
        let invalid_bytes = [0x74, 0x65, 0x73, 0x74, 0xFF, 0xFF, 0x2E, 0x74, 0x78, 0x74];
        let invalid_path = PathBuf::from(OsStr::from_bytes(&invalid_bytes));
        let name = EntryReference::from_lossy(invalid_path);
        assert_eq!("test\u{FFFD}\u{FFFD}.txt", name.as_str());

        // Test with invalid UTF-8 sequence at the start
        let invalid_bytes = [0xFF, 0x74, 0x65, 0x73, 0x74, 0x2E, 0x74, 0x78, 0x74];
        let invalid_path = PathBuf::from(OsStr::from_bytes(&invalid_bytes));
        let name = EntryReference::from_lossy(invalid_path);
        assert_eq!("\u{FFFD}test.txt", name.as_str());

        // Test with invalid UTF-8 sequence at the end
        let invalid_bytes = [0x74, 0x65, 0x73, 0x74, 0x2E, 0x74, 0x78, 0x74, 0xFF];
        let invalid_path = PathBuf::from(OsStr::from_bytes(&invalid_bytes));
        let name = EntryReference::from_lossy(invalid_path);
        assert_eq!("test.txt\u{FFFD}", name.as_str());
    }

    #[test]
    fn as_ref_implementations() {
        let name = EntryReference::from("test.txt");

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
