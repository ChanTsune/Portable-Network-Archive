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
            Utf8Component::ParentDir => {
                if buf.parent().is_some() {
                    buf.pop();
                } else {
                    buf.push(c);
                }
            }
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
        assert_eq!("../a.txt", normalize_utf8path("../a.txt"));
    }
}
