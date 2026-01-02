use crate::utils::BsdGlobPatterns;

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

    /// Returns `true` if the given path matches an explicit exclude pattern.
    ///
    /// This is used to determine whether to prune directory traversal.
    /// Unlike `excluded()`, this does NOT consider include pattern mismatches,
    /// because we need to traverse directories to find files that match include patterns.
    #[inline]
    pub(crate) fn explicitly_excluded(&self, s: impl AsRef<str>) -> bool {
        self.exclude.matches_exclusion(s.as_ref())
    }
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

    #[test]
    fn path_filter_include_txt_files() {
        let include = ["**/*.txt"];
        let filter = PathFilter::new(include, EMPTY_PATTERNS);

        // Should NOT be excluded (matches include pattern)
        assert!(
            !filter.excluded("create_with_include/in/raw/empty.txt"),
            "empty.txt should NOT be excluded"
        );
        assert!(
            !filter.excluded("create_with_include/in/raw/text.txt"),
            "text.txt should NOT be excluded"
        );
        assert!(
            !filter.excluded("dir/file.txt"),
            "dir/file.txt should NOT be excluded"
        );

        // Should be excluded (doesn't match include pattern)
        assert!(
            filter.excluded("create_with_include/in/raw/images/icon.png"),
            "icon.png should be excluded"
        );
        assert!(
            filter.excluded("dir/file.png"),
            "dir/file.png should be excluded"
        );
    }

    #[test]
    fn path_filter_include_with_directory_entry() {
        // Test what happens with directory entries (which don't end in .txt)
        let include = ["**/*.txt"];
        let filter = PathFilter::new(include, EMPTY_PATTERNS);

        // Directory entries won't match **/*.txt pattern, so they'll be excluded
        // This is the bug! Directories are being excluded which prevents traversal
        assert!(
            filter.excluded("create_with_include/in/raw"),
            "directory entry is excluded because it doesn't match **/*.txt"
        );
    }
}
