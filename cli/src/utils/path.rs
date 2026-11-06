use std::{env, fmt, path::Path};

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

pub(crate) fn is_part_path(path: &Path) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_only_numbered_part_names() {
        for part in ["foo.part1.pna", "foo.part23.PNA", "foo.part1"] {
            assert!(is_part_path(Path::new(part)), "{part}");
        }
        for regular in ["foo.pna", "foo.partial.pna", "foo.part.pna", "foo.partx"] {
            assert!(!is_part_path(Path::new(regular)), "{regular}");
        }
    }
}
