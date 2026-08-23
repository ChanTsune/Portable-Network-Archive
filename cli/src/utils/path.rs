use std::{
    env, fmt,
    path::{Path, PathBuf},
};

pub(crate) trait PathPartExt {
    fn with_part(&self, n: usize) -> PathBuf;
    fn remove_part(&self) -> PathBuf;
    fn is_part(&self) -> bool;
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

    #[inline]
    fn is_part(&self) -> bool {
        is_part_path(self)
    }
}

#[inline]
fn is_part_path(path: &Path) -> bool {
    fn is_part_component(component: &std::ffi::OsStr) -> bool {
        let component = component.to_string_lossy();
        component.strip_prefix("part").is_some_and(|number| {
            !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
        })
    }

    let Some(extension) = path.extension() else {
        return false;
    };
    if extension.eq_ignore_ascii_case("pna") {
        Path::new(path.file_stem().unwrap_or_default())
            .extension()
            .is_some_and(is_part_component)
    } else {
        is_part_component(extension)
    }
}

#[inline]
fn with_part_n<P: AsRef<Path>>(p: P, n: usize) -> PathBuf {
    #[inline]
    fn with_ext(p: &Path, n: usize) -> PathBuf {
        let Some(file_stem) = p.file_stem() else {
            return PathBuf::from(p);
        };
        let name = Path::new(file_stem);
        if let Some(ext) = p.extension() {
            if ext.eq_ignore_ascii_case("pna") {
                name.with_extension(format!("part{n}.{}", ext.to_string_lossy()))
            } else {
                name.with_extension(format!("part{n}"))
            }
        } else {
            name.with_extension(format!("part{n}"))
        }
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
            if extension.to_string_lossy().starts_with("part") {
                PathBuf::from(file_name.file_stem().unwrap())
            } else {
                let stem = PathBuf::from(file_name.file_stem().unwrap());
                if let Some(may) = stem.extension() {
                    if may.to_string_lossy().starts_with("part") {
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

#[derive(Clone, Debug)]
pub(crate) struct PathWithCwd<'a> {
    path: &'a Path,
}

impl fmt::Display for PathWithCwd<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_absolute() {
            return self.path.display().fmt(f);
        }
        match env::current_dir() {
            Ok(cwd) => cwd.join(self.path).display().fmt(f),
            Err(e) => write!(f, "{} (cwd unavailable: {})", self.path.display(), e),
        }
    }
}

impl<'a> PathWithCwd<'a> {
    #[inline]
    pub(crate) const fn new(path: &'a Path) -> Self {
        Self { path }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn non_part_to_part_with_extension() {
        assert_eq!(with_part_n("a.pna", 1), Path::new("a.part1.pna"));
        assert_eq!(
            with_part_n("parent/a.pna", 1),
            Path::new("parent/a.part1.pna")
        );
    }

    #[test]
    fn part_to_part_with_extension() {
        assert_eq!(with_part_n("a.part1.pna", 2), Path::new("a.part2.pna"));
        assert_eq!(
            with_part_n("parent/a.part1.pna", 2),
            Path::new("parent/a.part2.pna")
        );
    }

    #[test]
    fn non_part_to_part_without_extension() {
        assert_eq!(with_part_n("a", 1), Path::new("a.part1"));
        assert_eq!(with_part_n("parent/a", 1), Path::new("parent/a.part1"));
    }

    #[test]
    fn part_to_part_without_extension() {
        assert_eq!(with_part_n("a.part1", 2), Path::new("a.part2"));
        assert_eq!(
            with_part_n("parent/a.part1", 2),
            Path::new("parent/a.part2")
        );
    }

    #[test]
    fn remove_part_name_with_extension() {
        assert_eq!(remove_part_n("foo.pna"), Path::new("foo.pna"));
        assert_eq!(remove_part_n("dir/foo.pna"), Path::new("dir/foo.pna"));
        assert_eq!(remove_part_n("foo.part1.pna"), Path::new("foo.pna"));
        assert_eq!(remove_part_n("dir/foo.part1.pna"), Path::new("dir/foo.pna"));
    }

    #[test]
    fn remove_part_name_without_extension() {
        assert_eq!(remove_part_n("foo"), Path::new("foo"));
        assert_eq!(remove_part_n("dir/foo"), Path::new("dir/foo"));

        assert_eq!(remove_part_n("foo.part1"), Path::new("foo"));
        assert_eq!(remove_part_n("dir/foo.part1"), Path::new("dir/foo"));
    }

    #[test]
    fn detects_only_numbered_part_names() {
        for part in ["foo.part1.pna", "foo.part23.PNA", "foo.part1"] {
            assert!(Path::new(part).is_part(), "{part}");
        }
        for regular in ["foo.pna", "foo.partial.pna", "foo.part.pna", "foo.partx"] {
            assert!(!Path::new(regular).is_part(), "{regular}");
        }
    }
}
