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
