use camino::{Utf8Component, Utf8Path, Utf8PathBuf};

#[inline]
pub(crate) fn normalize_utf8path(path: impl AsRef<Utf8Path>) -> Utf8PathBuf {
    let path = path.as_ref();
    let mut components = path.components().peekable();
    let mut buf = if let Some(p @ Utf8Component::Prefix(..)) = components.peek() {
        let buf = Utf8PathBuf::from(p);
        components.next();
        buf
    } else {
        Utf8PathBuf::new()
    };
    for c in components {
        match c {
            Utf8Component::Prefix(_) => unreachable!(),
            Utf8Component::RootDir => buf.push(c),
            Utf8Component::CurDir => (),
            Utf8Component::ParentDir => match buf.components().next_back() {
                Some(Utf8Component::Normal(_)) => {
                    buf.pop();
                }
                Some(Utf8Component::ParentDir) | None => buf.push(c),
                Some(Utf8Component::RootDir | Utf8Component::Prefix(_)) => {}
                Some(Utf8Component::CurDir) => unreachable!("normalized path must not contain '.'"),
            },
            Utf8Component::Normal(_) => buf.push(c),
        }
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize() {
        assert_eq!("", normalize_utf8path(""));
        assert_eq!("a.txt", normalize_utf8path("a.txt"));
        assert_eq!("/a.txt", normalize_utf8path("/a.txt"));
        assert_eq!("a.txt", normalize_utf8path("./a.txt"));
        assert_eq!("a.txt", normalize_utf8path("a/../a.txt"));
        assert_eq!("a.txt", normalize_utf8path("a/b/../../a.txt"));
        assert_eq!("../a.txt", normalize_utf8path("../a.txt"));
        assert_eq!("../..", normalize_utf8path("../.."));
        assert_eq!("../../a.txt", normalize_utf8path("../../a.txt"));
        assert_eq!("../a.txt", normalize_utf8path("a/../../a.txt"));
        assert_eq!("a/a.txt", normalize_utf8path("a/b/./../a.txt"));
        assert_eq!("/", normalize_utf8path("/"));
        assert_eq!("/a/b", normalize_utf8path("/a//b///"));
        assert_eq!("a/b", normalize_utf8path("a//b///"));
        assert_eq!("a/b", normalize_utf8path("a/b/"));
        assert_eq!("a", normalize_utf8path("a/."));
        assert_eq!("", normalize_utf8path("."));
        assert_eq!("..", normalize_utf8path(".."));
        assert_eq!("/", normalize_utf8path("/.."));
    }
}
