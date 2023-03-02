use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::path::{Component, PathBuf};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct ItemName(String);

impl From<&str> for ItemName {
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
    fn from(value: &str) -> Self {
        let buf = PathBuf::from(value);
        let buf = buf
            .components()
            .filter_map(|c| match c {
                Component::Prefix(_)
                | Component::RootDir
                | Component::CurDir
                | Component::ParentDir => None,
                Component::Normal(p) => Some(p),
            })
            .map(|i| i.to_string_lossy().to_string())
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
