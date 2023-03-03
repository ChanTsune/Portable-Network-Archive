use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::path::{Component, PathBuf};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ItemName(String);

impl<T: ?Sized + AsRef<OsStr>> From<&T> for ItemName {
    /// # Examples
    /// ```
    /// use libpna::ItemName;
    ///
    /// assert_eq!(ItemName::from("test.txt"), ItemName::from("test.txt"));
    ///
    /// assert_eq!(ItemName::from("/test.txt"), ItemName::from("test.txt"));
    ///
    /// assert_eq!(ItemName::from("./test.txt"), ItemName::from("test.txt"));
    ///
    /// assert_eq!(ItemName::from("../test.txt"), ItemName::from("test.txt"));
    /// ```
    fn from(value: &T) -> Self {
        Self::from(PathBuf::from(value))
    }
}

impl From<PathBuf> for ItemName {
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

impl Display for ItemName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for ItemName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_root() {
        assert_eq!(ItemName::from("/test.txt"), ItemName::from("test.txt"));
        assert_eq!(
            ItemName::from("/test/test.txt"),
            ItemName::from("test/test.txt")
        );
    }

    #[test]
    fn remove_last() {
        assert_eq!(ItemName::from("test/"), ItemName::from("test"));
        assert_eq!(ItemName::from("test/test/"), ItemName::from("test/test"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn remove_prefix() {
        assert_eq!(ItemName::from("C:\\test.txt"), ItemName::from("test.txt"));
        assert_eq!(
            ItemName::from("C:\\test\\test.txt"),
            ItemName::from("test/test.txt")
        );
    }
}
