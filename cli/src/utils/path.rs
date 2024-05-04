use std::path::{Path, PathBuf};

pub(crate) trait PathPartExt {
    fn with_part(&self, n: usize) -> Option<PathBuf>;
    fn remove_part(&self) -> Option<PathBuf>;
}

impl PathPartExt for Path {
    #[inline]
    fn with_part(&self, n: usize) -> Option<PathBuf> {
        with_part_n(self, n)
    }

    #[inline]
    fn remove_part(&self) -> Option<PathBuf> {
        remove_part_n(self)
    }
}

#[inline]
pub(crate) fn with_part_n<P: AsRef<Path>>(p: P, n: usize) -> Option<PathBuf> {
    #[inline]
    fn with_ext(p: &Path, n: usize) -> Option<PathBuf> {
        let name = PathBuf::from(p.file_stem()?);
        if let Some(ext) = p.extension() {
            if ext.eq_ignore_ascii_case("pna") {
                Some(name.with_extension(format!("part{n}.{}", ext.to_string_lossy())))
            } else {
                Some(name.with_extension(format!("part{n}")))
            }
        } else {
            Some(name.with_extension(format!("part{n}")))
        }
    }
    let p = p.as_ref();
    if let Some(parent) = p.parent() {
        with_ext(p, n).map(|i| parent.join(i))
    } else {
        with_ext(p, n)
    }
}

#[inline]
pub(crate) fn remove_part_n<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    #[inline]
    fn inner(path: &Path) -> Option<PathBuf> {
        let parent = path.parent();
        let file_name = PathBuf::from(path.file_name()?);
        let removed = if let Some(extension) = file_name.extension() {
            if extension.to_str().unwrap().starts_with("part") {
                PathBuf::from(file_name.file_stem().unwrap())
            } else {
                let stem = PathBuf::from(file_name.file_stem().unwrap());
                if let Some(may) = stem.extension() {
                    if may.to_str().unwrap().starts_with("part") {
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
        Some(if let Some(parent) = parent {
            parent.join(removed)
        } else {
            removed
        })
    }
    inner(path.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn non_part_to_part_with_extension() {
        assert_eq!(with_part_n("a.pna", 1), Some(PathBuf::from("a.part1.pna")));
        assert_eq!(
            with_part_n("parent/a.pna", 1),
            Some(PathBuf::from("parent/a.part1.pna"))
        );
    }

    #[test]
    fn part_to_part_with_extension() {
        assert_eq!(
            with_part_n("a.part1.pna", 2),
            Some(PathBuf::from("a.part2.pna"))
        );
        assert_eq!(
            with_part_n("parent/a.part1.pna", 2),
            Some(PathBuf::from("parent/a.part2.pna"))
        );
    }

    #[test]
    fn non_part_to_part_without_extension() {
        assert_eq!(with_part_n("a", 1), Some(PathBuf::from("a.part1")));
        assert_eq!(
            with_part_n("parent/a", 1),
            Some(PathBuf::from("parent/a.part1"))
        );
    }

    #[test]
    fn part_to_part_without_extension() {
        assert_eq!(with_part_n("a.part1", 2), Some(PathBuf::from("a.part2")));
        assert_eq!(
            with_part_n("parent/a.part1", 2),
            Some(PathBuf::from("parent/a.part2"))
        );
    }

    #[test]
    fn remove_part_name_with_extension() {
        assert_eq!(remove_part_n("foo.pna").unwrap(), PathBuf::from("foo.pna"));
        assert_eq!(
            remove_part_n("dir/foo.pna").unwrap(),
            PathBuf::from("dir/foo.pna")
        );

        assert_eq!(
            remove_part_n("foo.part1.pna").unwrap(),
            PathBuf::from("foo.pna")
        );
        assert_eq!(
            remove_part_n("dir/foo.part1.pna").unwrap(),
            PathBuf::from("dir/foo.pna")
        );
    }

    #[test]
    fn remove_part_name_without_extension() {
        assert_eq!(remove_part_n("foo").unwrap(), PathBuf::from("foo"));
        assert_eq!(remove_part_n("dir/foo").unwrap(), PathBuf::from("dir/foo"));

        assert_eq!(remove_part_n("foo.part1").unwrap(), PathBuf::from("foo"));
        assert_eq!(
            remove_part_n("dir/foo.part1").unwrap(),
            PathBuf::from("dir/foo")
        );
    }
}
