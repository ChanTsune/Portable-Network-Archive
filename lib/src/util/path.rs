use std::path::{Component, Path, PathBuf};

#[inline]
pub(crate) fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    let mut components = path.components().peekable();
    let mut buf = if let Some(c @ Component::Prefix(..)) = components.peek() {
        let buf = PathBuf::from(c);
        components.next();
        buf
    } else {
        PathBuf::new()
    };
    for c in components {
        match c {
            Component::Prefix(_) => unreachable!(),
            Component::RootDir => buf.push(c),
            Component::CurDir => (),
            Component::ParentDir => match buf.components().next_back() {
                Some(Component::Normal(_)) => {
                    buf.pop();
                }
                Some(Component::ParentDir) | None => buf.push(c),
                Some(Component::RootDir | Component::Prefix(_)) => {}
                Some(Component::CurDir) => {
                    buf.pop();
                    buf.push(c);
                }
            },
            Component::Normal(_) => buf.push(c),
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    #[test]
    fn normalize() {
        assert_eq!(OsStr::new(""), normalize_path(""));
        assert_eq!(OsStr::new("a.txt"), normalize_path("a.txt"));
        assert_eq!(OsStr::new("/a.txt"), normalize_path("/a.txt"));
        assert_eq!(OsStr::new("a.txt"), normalize_path("./a.txt"));
        assert_eq!(OsStr::new("a.txt"), normalize_path("a/../a.txt"));
        assert_eq!(OsStr::new("a.txt"), normalize_path("a/b/../../a.txt"));
        assert_eq!(OsStr::new("../a.txt"), normalize_path("../a.txt"));
        assert_eq!(OsStr::new("../.."), normalize_path("../.."));
        assert_eq!(OsStr::new("../../a.txt"), normalize_path("../../a.txt"));
        assert_eq!(OsStr::new("../a.txt"), normalize_path("a/../../a.txt"));
        assert_eq!(OsStr::new("a/a.txt"), normalize_path("a/b/./../a.txt"));
        assert_eq!(OsStr::new("/"), normalize_path("/"));
        assert_eq!(OsStr::new("/a/b"), normalize_path("/a//b///"));
        assert_eq!(OsStr::new("a/b"), normalize_path("a//b///"));
        assert_eq!(OsStr::new("a/b"), normalize_path("a/b/"));
        assert_eq!(OsStr::new("a"), normalize_path("a/."));
        assert_eq!(OsStr::new(""), normalize_path("."));
        assert_eq!(OsStr::new(".."), normalize_path(".."));
        assert_eq!(OsStr::new("/"), normalize_path("/.."));
    }
}
