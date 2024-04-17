pub(crate) mod fs;
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

pub trait Let<T> {
    fn let_ref<U, F: FnOnce(&T) -> U>(&self, f: F);
}

impl<T> Let<T> for Option<T> {
    #[inline]
    fn let_ref<U, F: FnOnce(&T) -> U>(&self, f: F) {
        if let Some(t) = self.as_ref() {
            f(t);
        }
    }
}
