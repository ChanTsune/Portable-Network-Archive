pub(crate) mod fs;
mod io;
mod path;

pub(crate) use path::*;
use std::path::Path;

pub(crate) struct GlobPatterns(Vec<glob::Pattern>);

impl GlobPatterns {
    #[inline]
    pub(crate) fn new<I: IntoIterator<Item = S>, S: AsRef<str>>(
        patterns: I,
    ) -> Result<Self, glob::PatternError> {
        Ok(Self(
            patterns
                .into_iter()
                .map(|pattern| glob::Pattern::new(pattern.as_ref()))
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub(crate) fn matches_any_path(&self, path: &Path) -> bool {
        self.0.iter().any(|glob| glob.matches_path(path))
    }
}

impl From<Vec<glob::Pattern>> for GlobPatterns {
    #[inline]
    fn from(value: Vec<glob::Pattern>) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn glob_any_empty() {
        let globs = GlobPatterns::from(Vec::new());
        assert!(!globs.matches_any_path("some".as_ref()));
    }

    #[test]
    fn glob_any() {
        let globs = GlobPatterns::new(vec!["path/**"]).unwrap();
        assert!(globs.matches_any_path("path/foo.pna".as_ref()));
    }
}
