use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryName(String);

impl EntryName {
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }

    pub fn as_os_str(&self) -> &OsStr {
        self.as_ref()
    }

    pub fn as_path(&self) -> &Path {
        self.as_ref()
    }
}

impl<T: ?Sized + AsRef<OsStr>> From<&T> for EntryName {
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
    fn from(value: &T) -> Self {
        Self::from(PathBuf::from(value))
    }
}

impl From<PathBuf> for EntryName {
    fn from(value: PathBuf) -> Self {
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for EntryName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl AsRef<OsStr> for EntryName {
    fn as_ref(&self) -> &OsStr {
        self.0.as_ref()
    }
}

impl AsRef<Path> for EntryName {
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
