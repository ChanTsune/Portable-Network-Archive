use super::read_paths;
use crate::utils::{BsdGlobPatterns, VCS_FILES};
use std::{io, path::PathBuf};

/// A filter for paths based on include and exclude glob patterns.
#[derive(Clone, Debug)]
pub(crate) struct PathFilter<'a> {
    include: BsdGlobPatterns<'a>,
    exclude: BsdGlobPatterns<'a>,
}

impl<'a> PathFilter<'a> {
    #[inline]
    pub(crate) fn new(
        include: impl Into<BsdGlobPatterns<'a>>,
        exclude: impl Into<BsdGlobPatterns<'a>>,
    ) -> Self {
        Self {
            include: include.into(),
            exclude: exclude.into(),
        }
    }

    /// Returns `true` if the given path should be excluded.
    ///
    /// A path is excluded if it matches any of the `exclude` patterns,
    /// or if `include` patterns are provided and the path does not match any of them.
    /// Exclusion patterns take precedence over inclusion patterns.
    #[inline]
    pub(crate) fn excluded(&self, s: impl AsRef<str>) -> bool {
        let s = s.as_ref();
        self.exclude.matches_exclusion(s) || !self.include.matches_inclusion(s)
    }
}

#[inline]
pub(crate) fn new_path_filter<'a>(
    include_patterns: &'a [String],
    exclude_patterns: &'a mut Vec<String>,
    exclude_from: Option<PathBuf>,
    null_separate: bool,
    exclude_vcs: bool,
) -> io::Result<PathFilter<'a>> {
    if let Some(p) = exclude_from {
        exclude_patterns.extend(read_paths(p, null_separate)?);
    }
    let vcs_patterns = exclude_vcs
        .then(|| VCS_FILES.iter().copied())
        .into_iter()
        .flatten();
    Ok(PathFilter::new(
        include_patterns.iter().map(|s| s.as_str()),
        exclude_patterns
            .iter()
            .map(|s| s.as_str())
            .chain(vcs_patterns),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY_PATTERNS: [&str; 0] = [];
    #[test]
    fn path_filter_empty() {
        let filter = PathFilter::new(EMPTY_PATTERNS, EMPTY_PATTERNS);
        assert!(!filter.excluded("a/b/c"));
    }

    #[test]
    fn path_filter_exclude() {
        let exclude = ["a/*"];
        let filter = PathFilter::new(EMPTY_PATTERNS, exclude);
        assert!(filter.excluded("a/b/c"));
    }

    #[test]
    fn path_filter_include_precedence() {
        let include = ["a/*/c"];
        let exclude = ["a/*"];
        let filter = PathFilter::new(include, exclude);
        assert!(filter.excluded("a/b/c"));

        let exclude = ["a/*/c"];
        let filter = PathFilter::new(include, exclude);
        assert!(filter.excluded("a/b/c"));
    }
}
