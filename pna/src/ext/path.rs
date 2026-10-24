//! Split-archive path naming for PNA archives.
use super::private;
use std::path::{Path, PathBuf};

/// Path naming for split archive parts.
///
/// [`with_part`](Self::with_part) inserts a `.part{n}` suffix before the
/// final extension, and [`remove_part`](Self::remove_part) removes it.
pub trait PathPartExt: private::Sealed {
    /// Returns a path with a `.part{n}` suffix inserted before the final
    /// extension, or appended if there is no extension.
    ///
    /// `n` is the one-based part number. An existing `.part{n}` component is
    /// replaced when it is the final extension or immediately precedes the
    /// final extension.
    ///
    /// # Examples
    ///
    /// ```
    /// use pna::prelude::PathPartExt;
    /// use std::path::Path;
    ///
    /// assert_eq!(
    ///     Path::new("archive.pna").with_part(1),
    ///     Path::new("archive.part1.pna")
    /// );
    /// ```
    fn with_part(&self, n: usize) -> PathBuf;

    /// Returns a path with a `.part{n}` suffix removed.
    ///
    /// The suffix may be the final extension or immediately precede the final
    /// extension. If no such suffix is present, the path is returned unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// use pna::prelude::PathPartExt;
    /// use std::path::Path;
    ///
    /// assert_eq!(
    ///     Path::new("archive.part1.pna").remove_part(),
    ///     Path::new("archive.pna")
    /// );
    /// ```
    fn remove_part(&self) -> PathBuf;
}

impl PathPartExt for Path {
    #[inline]
    fn with_part(&self, n: usize) -> PathBuf {
        with_part_n(self, n)
    }

    #[inline]
    fn remove_part(&self) -> PathBuf {
        remove_part_n(self)
    }
}

#[inline]
fn is_part_n(ext: &std::ffi::OsStr) -> bool {
    match ext.as_encoded_bytes().strip_prefix(b"part") {
        Some(rest) => !rest.is_empty() && rest.iter().all(|b| b.is_ascii_digit()),
        None => false,
    }
}

#[inline]
fn with_part_n<P: AsRef<Path>>(p: P, n: usize) -> PathBuf {
    #[inline]
    fn with_ext(p: &Path, n: usize) -> PathBuf {
        if let Some(file_name) = p.file_name() {
            let file_name = Path::new(file_name);
            if let Some(extension) = file_name
                .extension()
                .filter(|extension| !extension.is_empty())
                && is_part_n(extension)
            {
                return file_name.with_extension(format!("part{n}"));
            }
        }
        let p = remove_part_n(p);
        let Some(file_name) = p.file_name() else {
            return p;
        };
        let file_name = Path::new(file_name);
        let Some(extension) = file_name
            .extension()
            .filter(|extension| !extension.is_empty())
        else {
            return file_name.with_extension(format!("part{n}"));
        };
        let mut part = std::ffi::OsString::from(format!("part{n}."));
        part.push(extension);
        file_name.with_extension(part)
    }
    let p = p.as_ref();
    if let Some(parent) = p.parent() {
        parent.join(with_ext(p, n))
    } else {
        with_ext(p, n)
    }
}

#[inline]
fn remove_part_n<P: AsRef<Path>>(path: P) -> PathBuf {
    #[inline]
    fn inner(path: &Path) -> PathBuf {
        let Some(file_name) = path.file_name() else {
            return PathBuf::from(path);
        };
        let parent = path.parent();
        let file_name = PathBuf::from(file_name);
        let removed = if let Some(extension) = file_name.extension() {
            if is_part_n(extension) {
                PathBuf::from(file_name.file_stem().unwrap())
            } else {
                let stem = PathBuf::from(file_name.file_stem().unwrap());
                if let Some(may) = stem.extension() {
                    if is_part_n(may) {
                        stem.with_extension(extension)
                    } else {
                        file_name
                    }
                } else {
                    file_name
                }
            }
        } else {
            file_name
        };
        if let Some(parent) = parent {
            parent.join(removed)
        } else {
            removed
        }
    }
    inner(path.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_part_adds_part_before_pna_extension() {
        assert_eq!(Path::new("a.pna").with_part(1), Path::new("a.part1.pna"));
        assert_eq!(
            Path::new("parent/a.pna").with_part(1),
            Path::new("parent/a.part1.pna")
        );
        assert_eq!(
            Path::new("my.archive.pna").with_part(1),
            Path::new("my.archive.part1.pna")
        );
    }

    #[test]
    fn with_part_replaces_existing_part_before_pna_extension() {
        assert_eq!(
            Path::new("a.part1.pna").with_part(2),
            Path::new("a.part2.pna")
        );
        assert_eq!(
            Path::new("parent/a.part1.pna").with_part(2),
            Path::new("parent/a.part2.pna")
        );
    }

    #[test]
    fn with_part_replaces_final_part_extension() {
        assert_eq!(
            Path::new("a.part1.part2").with_part(3),
            Path::new("a.part1.part3")
        );
        assert_eq!(
            Path::new("a.part1.part2").with_part(2),
            Path::new("a.part1.part2")
        );
        assert_eq!(
            Path::new("a.part1.part2.pna").with_part(3),
            Path::new("a.part1.part3.pna")
        );
    }

    #[test]
    fn with_part_appends_part_without_extension() {
        assert_eq!(Path::new("a").with_part(1), Path::new("a.part1"));
        assert_eq!(
            Path::new("parent/a").with_part(1),
            Path::new("parent/a.part1")
        );
    }

    #[test]
    fn with_part_replaces_existing_part_without_extension() {
        assert_eq!(Path::new("a.part1").with_part(2), Path::new("a.part2"));
        assert_eq!(
            Path::new("parent/a.part1").with_part(2),
            Path::new("parent/a.part2")
        );
    }

    #[test]
    fn with_part_preserves_non_pna_extensions() {
        assert_eq!(
            Path::new("archive.txt").with_part(1),
            Path::new("archive.part1.txt")
        );
        assert_eq!(
            Path::new("archive.tar.gz").with_part(1),
            Path::new("archive.tar.part1.gz")
        );
        assert_eq!(
            Path::new("archive.part1.txt").with_part(2),
            Path::new("archive.part2.txt")
        );
    }

    #[test]
    fn remove_part_restores_pna_extension() {
        assert_eq!(Path::new("foo.pna").remove_part(), Path::new("foo.pna"));
        assert_eq!(
            Path::new("dir/foo.pna").remove_part(),
            Path::new("dir/foo.pna")
        );
        assert_eq!(
            Path::new("foo.part1.pna").remove_part(),
            Path::new("foo.pna")
        );
        assert_eq!(
            Path::new("dir/foo.part1.pna").remove_part(),
            Path::new("dir/foo.pna")
        );
        assert_eq!(
            Path::new("my.archive.part1.pna").remove_part(),
            Path::new("my.archive.pna")
        );
    }

    #[test]
    fn remove_part_restores_extensionless_name() {
        assert_eq!(Path::new("foo").remove_part(), Path::new("foo"));
        assert_eq!(Path::new("dir/foo").remove_part(), Path::new("dir/foo"));
        assert_eq!(Path::new("foo.part1").remove_part(), Path::new("foo"));
        assert_eq!(
            Path::new("dir/foo.part1").remove_part(),
            Path::new("dir/foo")
        );
    }

    #[test]
    fn remove_part_restores_non_pna_extensions() {
        assert_eq!(
            Path::new("archive.part1.txt").remove_part(),
            Path::new("archive.txt")
        );
        assert_eq!(
            Path::new("archive.tar.part1.gz").remove_part(),
            Path::new("archive.tar.gz")
        );
    }

    #[test]
    fn remove_part_keeps_non_part_like_names() {
        assert_eq!(Path::new("foo.part").remove_part(), Path::new("foo.part"));
        assert_eq!(
            Path::new("foo.partial").remove_part(),
            Path::new("foo.partial")
        );
        assert_eq!(Path::new("foo.parts").remove_part(), Path::new("foo.parts"));
        assert_eq!(
            Path::new("foo.part1a").remove_part(),
            Path::new("foo.part1a")
        );
        assert_eq!(
            Path::new("foo.part.pna").remove_part(),
            Path::new("foo.part.pna")
        );
        assert_eq!(
            Path::new("foo.partial.pna").remove_part(),
            Path::new("foo.partial.pna")
        );
        assert_eq!(
            Path::new("dir/foo.partial.pna").remove_part(),
            Path::new("dir/foo.partial.pna")
        );
    }
}
