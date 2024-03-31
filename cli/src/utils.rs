pub(crate) mod fs;
mod path;

use glob::PatternError;
pub(crate) use path::*;
use std::path::Path;

pub(crate) struct GlobPatterns(Vec<glob::Pattern>);

impl GlobPatterns {
    #[inline]
    pub(crate) fn new<I: IntoIterator<Item = S>, S: AsRef<str>>(
        patterns: I,
    ) -> Result<Self, PatternError> {
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
