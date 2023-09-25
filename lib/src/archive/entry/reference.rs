use crate::util::try_to_string;
use std::borrow::Cow;
use std::path::{Component, Path};

/// A UTF-8 encoded entry reference.
///
/// ## Examples
/// ```
/// use libpna::EntryReference;
///
/// assert_eq!("uer/bin", EntryReference::try_from("uer/bin").unwrap().as_str());
/// assert_eq!("/user/bin", EntryReference::try_from("/user/bin").unwrap().as_str());
/// assert_eq!("/user/bin", EntryReference::try_from("/user/bin/").unwrap().as_str());
/// assert_eq!("../user/bin", EntryReference::try_from("../user/bin/").unwrap().as_str());
/// assert_eq!("/", EntryReference::try_from("/").unwrap().as_str());
/// ```
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryReference(String);

impl EntryReference {
    fn new(path: &Path) -> Result<Self, ()> {
        let has_root = path.has_root();
        let mut components = path.components();
        if has_root {
            components.next();
        };
        let p = components
            .map(|it| match it {
                Component::Prefix(p) => try_to_string(p.as_os_str()),
                Component::RootDir => unreachable!(),
                Component::CurDir => Ok(Cow::from(".")),
                Component::ParentDir => Ok(Cow::from("..")),
                Component::Normal(n) => try_to_string(n),
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ())?;
        let mut s = p.join("/");
        if has_root {
            s.insert(0, '/');
        };
        Ok(Self(s))
    }

    /// Extracts a string slice containing the entire [EntryReference].
    /// ## Examples
    /// Basic usage:
    /// ```
    /// use libpna::EntryReference;
    /// let r = EntryReference::try_from("foo").unwrap();
    ///
    /// assert_eq!("foo", r.as_str());
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl TryFrom<&str> for EntryReference {
    type Error = ();

    /// ## Examples
    /// ```
    /// use libpna::EntryReference;
    ///
    /// assert_eq!(EntryReference::try_from("/path/with/root"), EntryReference::try_from("/path/with/root"));
    /// ```
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value.as_ref())
    }
}

impl TryFrom<&Path> for EntryReference {
    type Error = ();

    /// ## Examples
    /// ```
    /// use std::path::Path;
    /// use libpna::EntryReference;
    ///
    /// let p = Path::new("path/to/file");
    /// assert_eq!(EntryReference::try_from(p), EntryReference::try_from("path/to/file"));
    /// ```
    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
