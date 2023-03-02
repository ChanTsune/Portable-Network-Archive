use std::ffi::OsStr;
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

#[derive(Clone, Eq, PartialEq, Debug)]
struct Name(String);

impl From<&str> for Name {
    fn from(value: &str) -> Self {
        let buf = PathBuf::from(value);
        let buf = buf
            .into_iter()
            .filter(|i| *i != OsStr::new(".") && *i != OsStr::new(".."))
            .map(|i| i.to_string_lossy().to_string())
            .collect::<Vec<_>>();
        Self(buf.join("/"))
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correct() {
        assert_eq!(Name::from("test.txt"), Name("test.txt".to_string()))
    }

    #[test]
    fn normalized() {
        assert_eq!(Name::from("./test.txt"), Name("test.txt".to_string()))
    }
}
