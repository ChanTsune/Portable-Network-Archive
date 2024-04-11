use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

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
        let mut components = path.components();
        if has_root {
            components.next();
        };
        let p = components
            .map(|it| match it {
                Utf8Component::Prefix(p) => p.as_str(),
                Utf8Component::RootDir => unreachable!(),
                Utf8Component::CurDir => ".",
                Utf8Component::ParentDir => "..",
                Utf8Component::Normal(n) => n,
            })
            .collect::<Vec<_>>();
        let mut s = p.join("/");
        if has_root {
            s.insert(0, '/');
        };
        Self(s)
    }

    #[inline]
    fn new_from_utf8(name: &str) -> Self {
        Self::new_from_utf8path(&Utf8PathBuf::from(name))
    }

    #[inline]
    fn new_from_path(path: &Path) -> Result<Self, ()> {
        let path = Utf8Path::from_path(path).ok_or(())?;
        Ok(Self::new_from_utf8path(path))
    }

    /// Extracts a string slice containing the entire [EntryReference].
    /// ## Examples
    /// Basic usage:
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
    /// let entry_name = EntryReference::from_lossy("foo.txt");
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
    /// let entry_name = EntryReference::from_lossy("test/foo.txt");
    /// assert_eq!(Path::new("test/foo.txt"), entry_name.as_path());
    /// ```
    #[inline]
    pub fn as_path(&self) -> &Path {
        self.0.as_ref()
    }

    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
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
    pub fn from_lossy<T: Into<PathBuf>>(p: T) -> Self {
        let path = p.into();
        let has_root = path.has_root();
        let mut components = path.components();
        if has_root {
            components.next();
        };
        let p = components
            .map(|it| match it {
                Component::Prefix(p) => p.as_os_str().to_string_lossy(),
                Component::RootDir => unreachable!(),
                Component::CurDir => Cow::from("."),
                Component::ParentDir => Cow::from(".."),
                Component::Normal(n) => n.to_string_lossy(),
            })
            .collect::<Vec<_>>();
        let mut s = p.join("/");
        if has_root {
            s.insert(0, '/');
        };
        Self(s)
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
        Self::new_from_utf8(value.as_ref())
    }
}

impl TryFrom<&Path> for EntryReference {
    type Error = ();

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

impl PartialEq<str> for EntryReference {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        PartialEq::eq(self.as_str(), other)
    }
}

impl PartialEq<&str> for EntryReference {
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
    #[inline]
    fn eq(&self, other: &EntryReference) -> bool {
        PartialEq::eq(self, &other.as_str())
    }
}
