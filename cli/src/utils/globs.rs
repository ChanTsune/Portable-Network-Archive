use bitflags::bitflags;
use std::path::Path;

#[derive(Clone, Debug)]
pub(crate) struct GlobPatterns<'s> {
    globs: globset::GlobSet,
    raw_patterns: Vec<&'s str>,
    matched: Vec<bool>,
}

impl<'s> GlobPatterns<'s> {
    #[inline]
    pub(crate) fn new<I: IntoIterator<Item = &'s str>>(
        patterns: I,
    ) -> Result<Self, globset::Error> {
        let mut builder = globset::GlobSet::builder();
        let mut raw_patterns = Vec::new();
        for pattern in patterns {
            let glob = globset::Glob::new(pattern)?;
            raw_patterns.push(pattern);
            builder.add(glob);
        }
        let globs = builder.build()?;
        Ok(Self {
            matched: vec![false; globs.len()],
            raw_patterns,
            globs,
        })
    }

    #[inline]
    pub(crate) fn matches_any<P: AsRef<Path>>(&mut self, s: P) -> bool {
        let indices = self.globs.matches(s);
        for idx in indices.iter() {
            self.matched[*idx] = true;
        }
        !indices.is_empty()
    }

    #[inline]
    pub(crate) fn ensure_all_matched(&self) -> anyhow::Result<()> {
        let mut any_unmatched = false;
        for (idx, &is_matched) in self.matched.iter().enumerate() {
            if !is_matched {
                any_unmatched = true;
                log::error!("'{}' not found in archive", self.raw_patterns[idx]);
            }
        }
        if any_unmatched {
            anyhow::bail!("from previous errors");
        }
        Ok(())
    }
}

/// Tar-compatible glob matcher that mirrors libarchive/bsdtar semantics.
///
/// Unlike `GlobPatterns`, this uses the project's `BsdGlobPattern` (which is
/// derived from libarchive's path matching) and also tracks which patterns
/// matched so we can surface the same "not found in archive" diagnostics.
#[derive(Clone, Debug)]
pub(crate) struct BsdGlobMatcher<'s> {
    patterns: Vec<BsdGlobPattern<'s>>,
    raw_patterns: Vec<&'s str>,
    matched: Vec<bool>,
    /// When true, patterns without glob meta do NOT match directory prefixes.
    /// This corresponds to bsdtar's -n/--no-recursive option.
    no_recursive: bool,
}

impl<'s> BsdGlobMatcher<'s> {
    #[inline]
    pub(crate) fn new<I: IntoIterator<Item = &'s str>>(patterns: I) -> Self {
        let raw_patterns: Vec<&'s str> = patterns.into_iter().collect();
        let patterns = raw_patterns
            .iter()
            .map(|p| BsdGlobPattern::new(p))
            .collect();
        let matched = vec![false; raw_patterns.len()];
        Self {
            patterns,
            raw_patterns,
            matched,
            no_recursive: false,
        }
    }

    /// Set the no-recursive mode. When enabled, patterns without glob meta
    /// do NOT match directory prefixes (bsdtar -n behavior).
    #[inline]
    pub(crate) fn with_no_recursive(mut self, no_recursive: bool) -> Self {
        self.no_recursive = no_recursive;
        self
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    fn pattern_matches_path(&self, idx: usize, path: &str) -> bool {
        if self.no_recursive {
            self.patterns[idx].match_inclusion(path)
        } else {
            self.patterns[idx].match_inclusion(path)
                || (!has_glob_meta(self.raw_patterns[idx])
                    && prefix_match(self.raw_patterns[idx], path))
        }
    }

    /// Returns true if any pattern matches the given path. Patterns without
    /// glob meta also match directory prefixes (e.g., pattern "dir" matches
    /// "dir/file"), unless `no_recursive` mode is enabled.
    #[inline]
    pub(crate) fn matches(&mut self, path: impl AsRef<str>) -> bool {
        let path = path.as_ref();
        let mut matched_any = false;
        for idx in 0..self.patterns.len() {
            if self.pattern_matches_path(idx, path) {
                self.matched[idx] = true;
                matched_any = true;
            }
        }
        matched_any
    }

    /// Returns true if any pattern matches the given path, regardless of
    /// whether the pattern has already been satisfied.
    #[inline]
    pub(crate) fn matches_any_pattern(&self, path: impl AsRef<str>) -> bool {
        let path = path.as_ref();
        (0..self.patterns.len()).any(|idx| self.pattern_matches_path(idx, path))
    }

    #[inline]
    pub(crate) fn mark_satisfied(&mut self, path: impl AsRef<str>) {
        let path = path.as_ref();
        for idx in 0..self.patterns.len() {
            if !self.matched[idx] && self.pattern_matches_path(idx, path) {
                self.matched[idx] = true;
            }
        }
    }

    #[inline]
    pub(crate) fn all_matched(&self) -> bool {
        self.matched.iter().all(|matched| *matched)
    }

    #[inline]
    pub(crate) fn ensure_all_matched(&self) -> anyhow::Result<()> {
        let mut any_unmatched = false;
        for (idx, &is_matched) in self.matched.iter().enumerate() {
            if !is_matched {
                any_unmatched = true;
                log::error!("'{}' not found in archive", self.raw_patterns[idx]);
            }
        }
        if any_unmatched {
            anyhow::bail!("from previous errors");
        }
        Ok(())
    }
}

#[inline]
fn has_glob_meta(pattern: &str) -> bool {
    pattern.contains(['*', '?', '[', '{'])
}

#[inline]
fn prefix_match(pattern: &str, path: &str) -> bool {
    // Normalize pattern by stripping trailing slash for bsdtar compatibility.
    // bsdtar treats "dir/" and "dir" identically for prefix matching.
    let pattern = pattern.strip_suffix('/').unwrap_or(pattern);
    if !path.starts_with(pattern) {
        return false;
    }
    // Exact match is already handled by match_inclusion; here we only care
    // about "pattern/…" forms.
    path.as_bytes()
        .get(pattern.len())
        .map(|next| *next == b'/')
        .unwrap_or(false)
}

/// BSD tar command like globs.
#[derive(Clone, Debug)]
pub(crate) struct BsdGlobPatterns<'a>(Vec<BsdGlobPattern<'a>>);

impl<'a> BsdGlobPatterns<'a> {
    #[inline]
    pub fn new(value: impl IntoIterator<Item = &'a str>) -> Self {
        Self(value.into_iter().map(BsdGlobPattern::new).collect())
    }

    #[inline]
    pub fn matches_exclusion(&self, s: impl AsRef<str>) -> bool {
        self.0.iter().any(|it| it.match_exclusion(s.as_ref()))
    }

    #[inline]
    /// Returns `true` if the path should be included.
    ///
    /// A path is considered for inclusion if it matches any of the provided glob patterns.
    /// If no patterns are provided, all paths are considered included (the function returns `true`).
    pub fn matches_inclusion(&self, s: impl AsRef<str>) -> bool {
        self.0.is_empty() || self.0.iter().any(|it| it.match_inclusion(s.as_ref()))
    }
}

impl<'a, T, I> From<I> for BsdGlobPatterns<'a>
where
    T: AsRef<str> + ?Sized + 'a,
    I: IntoIterator<Item = &'a T>,
{
    #[inline]
    fn from(value: I) -> Self {
        Self::new(value.into_iter().map(|it| it.as_ref()))
    }
}

/// BSD tar command like glob.
#[derive(Clone, Debug)]
pub(crate) struct BsdGlobPattern<'a> {
    pattern: &'a str,
}

impl<'a> BsdGlobPattern<'a> {
    #[inline]
    pub fn new(pattern: &'a str) -> Self {
        Self { pattern }
    }

    #[inline]
    pub fn match_exclusion(&self, s: &str) -> bool {
        archive_pathmatch(
            self.pattern,
            s,
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END,
        )
    }

    #[inline]
    pub fn match_inclusion(&self, s: &str) -> bool {
        archive_pathmatch(self.pattern, s, PathMatch::NO_ANCHOR_START)
    }
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub(crate) struct PathMatch: usize {
        /// Don't anchor at beginning unless the pattern starts with "^"
        const NO_ANCHOR_START = 1;
        /// Don't anchor at end unless the pattern ends with "$"
        const NO_ANCHOR_END = 2;
    }
}

/*
 * If s is pointing to "./", ".//", "./././" or the like, skip it.
 */
fn pm_slashskip(mut s: &str) -> &str {
    s = s.trim_start_matches('/');
    while let Some(rest) = s.strip_prefix("./") {
        s = rest.trim_start_matches('/');
    }
    if s == "." {
        s = &s[1..];
    }
    s
}

/*
 * Check whether a character 'c' is matched by a list specification [...]:
 *    * Leading '!' or '^' negates the class.
 *    * <char>-<char> is a range of characters
 *    * \<char> removes any special meaning for <char>
 *
 * Some interesting boundary cases:
 *   a-d-e is one range (a-d) followed by two single characters - and e.
 *   \a-\d is same as a-d
 *   a\-d is three single characters: a, d, -
 *   Trailing - is not special (so [a-] is two characters a and -).
 *   Initial - is not special ([a-] is same as [-a] is same as [\\-a])
 *   This function never sees a trailing \.
 *   [] always fails
 *   [!] always succeeds
 */
fn pm_list(mut class: &str, c: char, _flags: PathMatch) -> bool {
    let mut r#match = true;
    let mut nomatch = false;
    //
    // 	/* This will be used soon... */
    // 	(void)flags; /* UNUSED */
    //
    /* If this is a negated class, return success for nomatch. */
    if let Some(cls) = class.strip_prefix('!') {
        r#match = false;
        nomatch = true;
        class = cls;
    } else if let Some(cls) = class.strip_prefix('^') {
        r#match = false;
        nomatch = true;
        class = cls;
    };
    let mut chars = class.chars();

    let mut range_start = None;
    let mut next_range_start;
    while let Some(p) = chars.next() {
        next_range_start = None;
        match p {
            '-' => {
                /* Trailing or initial '-' is not special. */
                if range_start.is_none() || chars.as_str().is_empty() {
                    if p == c {
                        return r#match;
                    }
                } else {
                    let mut range_end = chars.next();
                    if range_end == Some('\\') {
                        range_end = chars.next();
                    }
                    if (range_start.is_some_and(|it| it <= c))
                        && (range_end.is_some_and(|it| c <= it))
                    {
                        return r#match;
                    }
                }
            }
            '\\' => {
                let p = chars.next();
                if p == Some(c) {
                    return r#match;
                }
                next_range_start = p; /* Possible start of range. */
            }
            _ => {
                if p == c {
                    return r#match;
                }
                next_range_start = Some(p); /* Possible start of range. */
            }
        }
        range_start = next_range_start;
    }
    nomatch
}

/* Main entry point. */
fn archive_pathmatch(mut p: &str, mut s: &str, mut flags: PathMatch) -> bool {
    /* Empty pattern only matches the empty string. */
    if p.is_empty() {
        return s.is_empty();
    }

    /* Leading '^' anchors the start of the pattern. */
    if let Some(_p) = p.strip_prefix('^') {
        flags &= !PathMatch::NO_ANCHOR_START;
        p = _p;
    }

    if p.starts_with('/') && !s.starts_with('/') {
        return false;
    }

    /* Certain patterns anchor implicitly. */
    if p.starts_with('*') || p.starts_with('/') {
        p = p.trim_start_matches('/');
        s = s.trim_start_matches('/');
        return pm(p, s, flags);
    }

    /* If start is unanchored, try to match start of each path element. */
    if flags.contains(PathMatch::NO_ANCHOR_START) {
        if let Some(_s) = s.strip_prefix('/') {
            s = _s;
        }
        loop {
            if pm(p, s, flags) {
                return true;
            }
            let Some((_, _s)) = s.split_once('/') else {
                break;
            };
            s = _s;
        }
        return false;
    }

    /* Default: Match from beginning. */
    pm(p, s, flags)
}

fn pm(mut p: &str, mut s: &str, flags: PathMatch) -> bool {
    // 	const char *end;
    //
    // 	/*
    // 	 * Ignore leading './', './/', '././', etc.
    // 	 */
    if let Some(_s) = s.strip_prefix("./") {
        s = pm_slashskip(_s);
    }
    if let Some(_p) = p.strip_prefix("./") {
        p = pm_slashskip(_p);
    }

    while let Some(c) = p.chars().next() {
        match c {
            '?' => {
                /* ? always succeeds, unless we hit end of 's' */
                if s.is_empty() {
                    return false;
                }
                p = skip_first_char(p);
                s = skip_first_char(s);
            }
            '*' => {
                /* "*" == "**" == "***" ... */
                p = p.trim_start_matches('*');
                /* Trailing '*' always succeeds. */
                if p.is_empty() {
                    return true;
                }
                while !s.is_empty() {
                    if archive_pathmatch(p, s, flags) {
                        return true;
                    }
                    s = skip_first_char(s);
                }
                return false;
            }
            '[' => {
                /*
                 * Find the end of the [...] character class,
                 * ignoring \] that might occur within the class.
                 */
                if let Some((l, r)) = split_once_unescaped(&p[1..]) {
                    /* We found [...], try to match it. */
                    if s.chars().next().is_some_and(|c| !pm_list(l, c, flags)) {
                        return false;
                    }
                    p = r;
                    s = skip_first_char(s);
                } else {
                    /* No final ']', so just match '['. */
                    if p.chars().next() != s.chars().next() {
                        return false;
                    }
                    p = skip_first_char(p);
                    s = skip_first_char(s);
                }
            }
            '\\' => {
                /* Trailing '\\' matches itself. */
                if p.len() == 1 {
                    if s.chars().next().is_some_and(|c| c != '\\') {
                        return false;
                    }
                } else {
                    p = skip_first_char(p);
                    if p.chars().next() != s.chars().next() {
                        return false;
                    }
                }
                p = skip_first_char(p);
                s = skip_first_char(s);
            }
            '/' => {
                if s.chars().next().is_some_and(|c| c != '/') {
                    return false;
                }
                /* Note: pattern "/\./" won't match "/";
                 * pm_slashskip() correctly stops at backslash. */
                p = pm_slashskip(p);
                s = pm_slashskip(s);
                if p.is_empty() && flags.contains(PathMatch::NO_ANCHOR_END) {
                    return true;
                }
            }
            '$' => {
                /* '$' is special only at end of pattern and only
                 * if PATHMATCH_NO_ANCHOR_END is specified. */
                if p.len() == 1 && flags.contains(PathMatch::NO_ANCHOR_END) {
                    /* "dir" == "dir/" == "dir/." */
                    return pm_slashskip(s).is_empty();
                }
                /* Otherwise, '$' is not special. */
                if p.chars().next() != s.chars().next() {
                    return false;
                }
                p = skip_first_char(p);
                s = skip_first_char(s);
            }
            _ => {
                if p.chars().next() != s.chars().next() {
                    return false;
                }
                p = skip_first_char(p);
                s = skip_first_char(s);
            }
        }
    }
    if s.starts_with('/') {
        if flags.contains(PathMatch::NO_ANCHOR_END) {
            return true;
        }
        /* "dir" == "dir/" == "dir/." */
        s = pm_slashskip(s);
    }
    s.is_empty()
}

fn split_once_unescaped(input: &str) -> Option<(&str, &str)> {
    let chars = input.char_indices().peekable();
    let mut last_escape = false;

    for (i, c) in chars {
        match c {
            '\\' => {
                last_escape = !last_escape;
            }
            ']' if !last_escape => {
                return Some((&input[..i], &input[i + 1..]));
            }
            _ => {
                last_escape = false;
            }
        }
    }

    None
}

fn skip_first_char(s: &str) -> &str {
    let mut chars = s.chars();
    let _ = chars.next();
    chars.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn glob_any_empty() {
        let mut globs = GlobPatterns::new([]).unwrap();
        assert!(!globs.matches_any("some"));
    }

    #[test]
    fn glob_asterisk() {
        let mut globs = GlobPatterns::new(["*"]).unwrap();
        assert!(globs.matches_any("same"));
        assert!(globs.matches_any("same/path"));
    }

    #[test]
    fn glob_suffix() {
        let mut globs = GlobPatterns::new(vec!["path/**"]).unwrap();
        assert!(globs.matches_any("path/foo.pna"));
        assert!(!globs.matches_any("foo/path"));
    }

    #[test]
    fn glob_prefix() {
        let mut globs = GlobPatterns::new(vec!["**/foo.pna"]).unwrap();
        assert!(globs.matches_any("path/foo.pna"));
        assert!(globs.matches_any("path/path/foo.pna"));
        assert!(!globs.matches_any("path/foo.pna/path"));
    }

    #[test]
    fn glob_middle_component() {
        let mut globs = GlobPatterns::new(vec!["usr/**/bin"]).unwrap();
        assert!(globs.matches_any("usr/local/bin"));
        assert!(globs.matches_any("usr/share/bin"));
    }

    #[test]
    fn test_normal_split() {
        assert_eq!(split_once_unescaped("abc]def"), Some(("abc", "def")));
    }

    #[test]
    fn test_escaped_bracket() {
        assert_eq!(split_once_unescaped("abc\\]def"), None);
    }

    #[test]
    fn test_second_bracket_splits() {
        assert_eq!(split_once_unescaped("a\\]b]c"), Some(("a\\]b", "c")));
    }

    #[test]
    fn test_escaped_first_bracket_splits_on_second() {
        assert_eq!(split_once_unescaped("\\]abc]def"), Some(("\\]abc", "def")));
    }

    #[test]
    fn test_bracket_at_start() {
        assert_eq!(split_once_unescaped("]abc"), Some(("", "abc")));
    }

    #[test]
    fn test_complex_escape_sequence() {
        assert_eq!(
            split_once_unescaped("abc\\]\\]def]x"),
            Some(("abc\\]\\]def", "x"))
        );
    }

    #[test]
    fn test_no_bracket() {
        assert_eq!(split_once_unescaped("no_brackets"), None);
    }

    #[test]
    fn archive_path_match() {
        assert!(archive_pathmatch("a/b/c", "a/b/c", PathMatch::empty()));
        assert!(!archive_pathmatch("a/b/", "a/b/c", PathMatch::empty()));
        assert!(!archive_pathmatch("a/b", "a/b/c", PathMatch::empty()));
        assert!(!archive_pathmatch("a/b/c", "a/b/", PathMatch::empty()));
        assert!(!archive_pathmatch("a/b/c", "a/b", PathMatch::empty()));

        // /* Null string and non-empty pattern returns false. */
        // assert!(! archive_pathmatch("a/b/c", NULL, PathMatch::empty()));
        // assert!(! archive_pathmatch_w(L"a/b/c", NULL, PathMatch::empty()));

        /* Empty pattern only matches empty string. */
        assert!(archive_pathmatch("", "", PathMatch::empty()));
        assert!(!archive_pathmatch("", "a", PathMatch::empty()));
        assert!(archive_pathmatch("*", "", PathMatch::empty()));
        assert!(archive_pathmatch("*", "a", PathMatch::empty()));
        assert!(archive_pathmatch("*", "abcd", PathMatch::empty()));
        /* SUSv2: * matches / */
        assert!(archive_pathmatch("*", "abcd/efgh/ijkl", PathMatch::empty()));
        assert!(archive_pathmatch(
            "abcd*efgh/ijkl",
            "abcd/efgh/ijkl",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "abcd***efgh/ijkl",
            "abcd/efgh/ijkl",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "abcd***/efgh/ijkl",
            "abcd/efgh/ijkl",
            PathMatch::empty()
        ));
        assert!(!archive_pathmatch("?", "", PathMatch::empty()));
        // assert!(!archive_pathmatch("?", "\0", PathMatch::empty()));
        assert!(archive_pathmatch("?", "a", PathMatch::empty()));
        assert!(!archive_pathmatch("?", "ab", PathMatch::empty()));
        assert!(archive_pathmatch("?", ".", PathMatch::empty()));
        assert!(archive_pathmatch("?", "?", PathMatch::empty()));
        assert!(archive_pathmatch("a", "a", PathMatch::empty()));
        assert!(!archive_pathmatch("a", "ab", PathMatch::empty()));
        assert!(!archive_pathmatch("a", "ab", PathMatch::empty()));
        assert!(archive_pathmatch("a?c", "abc", PathMatch::empty()));
        /* SUSv2: ? matches / */
        assert!(archive_pathmatch("a?c", "a/c", PathMatch::empty()));
        assert!(archive_pathmatch("a?*c*", "a/c", PathMatch::empty()));
        assert!(archive_pathmatch("*a*", "a/c", PathMatch::empty()));
        assert!(archive_pathmatch("*a*", "/a/c", PathMatch::empty()));
        assert!(archive_pathmatch("*a*", "defaaaaaaa", PathMatch::empty()));
        assert!(!archive_pathmatch("a*", "defghi", PathMatch::empty()));
        assert!(!archive_pathmatch("*a*", "defghi", PathMatch::empty()));

        /* Character classes */
        assert!(archive_pathmatch("abc[def", "abc[def", PathMatch::empty()));
        assert!(!archive_pathmatch(
            "abc[def]",
            "abc[def",
            PathMatch::empty()
        ));
        assert!(!archive_pathmatch("abc[def", "abcd", PathMatch::empty()));
        assert!(archive_pathmatch("abc[def]", "abcd", PathMatch::empty()));
        assert!(archive_pathmatch("abc[def]", "abce", PathMatch::empty()));
        assert!(archive_pathmatch("abc[def]", "abcf", PathMatch::empty()));
        assert!(!archive_pathmatch("abc[def]", "abcg", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d*f]", "abcd", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d*f]", "abc*", PathMatch::empty()));
        assert!(!archive_pathmatch(
            "abc[d*f]",
            "abcdefghi",
            PathMatch::empty()
        ));
        assert!(!archive_pathmatch(
            "abc[d*",
            "abcdefghi",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "abc[d*",
            "abc[defghi",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch("abc[d-f]", "abcd", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d-f]", "abce", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d-f]", "abcf", PathMatch::empty()));
        assert!(!archive_pathmatch("abc[d-f]", "abcg", PathMatch::empty()));
        assert!(!archive_pathmatch(
            "abc[d-fh-k]",
            "abca",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch("abc[d-fh-k]", "abcd", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d-fh-k]", "abce", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d-fh-k]", "abcf", PathMatch::empty()));
        assert!(!archive_pathmatch(
            "abc[d-fh-k]",
            "abcg",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch("abc[d-fh-k]", "abch", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d-fh-k]", "abci", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d-fh-k]", "abcj", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d-fh-k]", "abck", PathMatch::empty()));
        assert!(!archive_pathmatch(
            "abc[d-fh-k]",
            "abcl",
            PathMatch::empty()
        ));
        assert!(!archive_pathmatch(
            "abc[d-fh-k]",
            "abc-",
            PathMatch::empty()
        ));

        /* [] matches nothing, [!] is the same as ? */
        assert!(!archive_pathmatch(
            "abc[]efg",
            "abcdefg",
            PathMatch::empty()
        ));
        assert!(!archive_pathmatch(
            "abc[]efg",
            "abcqefg",
            PathMatch::empty()
        ));
        assert!(!archive_pathmatch("abc[]efg", "abcefg", PathMatch::empty()));
        assert!(archive_pathmatch(
            "abc[!]efg",
            "abcdefg",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "abc[!]efg",
            "abcqefg",
            PathMatch::empty()
        ));
        assert!(!archive_pathmatch(
            "abc[!]efg",
            "abcefg",
            PathMatch::empty()
        ));

        /* I assume: Trailing '-' is non-special. */
        assert!(!archive_pathmatch("abc[d-fh-]", "abcl", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d-fh-]", "abch", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d-fh-]", "abc-", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d-fh-]", "abc-", PathMatch::empty()));

        /* ']' can be backslash-quoted within a character class. */
        assert!(archive_pathmatch("abc[\\]]", "abc]", PathMatch::empty()));
        assert!(archive_pathmatch("abc[\\]d]", "abc]", PathMatch::empty()));
        assert!(archive_pathmatch("abc[\\]d]", "abcd", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d\\]]", "abc]", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d\\]]", "abcd", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d]e]", "abcde]", PathMatch::empty()));
        assert!(archive_pathmatch("abc[d\\]e]", "abc]", PathMatch::empty()));
        assert!(!archive_pathmatch(
            "abc[d\\]e]",
            "abcd]e",
            PathMatch::empty()
        ));
        assert!(!archive_pathmatch("abc[d]e]", "abc]", PathMatch::empty()));

        /* backslash-quoted chars can appear as either end of a range. */
        assert!(archive_pathmatch(
            "abc[\\d-f]gh",
            "abcegh",
            PathMatch::empty()
        ));
        assert!(!archive_pathmatch(
            "abc[\\d-f]gh",
            "abcggh",
            PathMatch::empty()
        ));
        assert!(!archive_pathmatch(
            "abc[\\d-f]gh",
            "abc\\gh",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "abc[d-\\f]gh",
            "abcegh",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "abc[\\d-\\f]gh",
            "abcegh",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "abc[\\d-\\f]gh",
            "abcegh",
            PathMatch::empty()
        ));
        /* backslash-quoted '-' isn't special. */
        assert!(!archive_pathmatch(
            "abc[d\\-f]gh",
            "abcegh",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "abc[d\\-f]gh",
            "abc-gh",
            PathMatch::empty()
        ));

        /* Leading '!' negates a character class. */
        assert!(!archive_pathmatch("abc[!d]", "abcd", PathMatch::empty()));
        assert!(archive_pathmatch("abc[!d]", "abce", PathMatch::empty()));
        assert!(archive_pathmatch("abc[!d]", "abcc", PathMatch::empty()));
        assert!(!archive_pathmatch("abc[!d-z]", "abcq", PathMatch::empty()));
        assert!(archive_pathmatch(
            "abc[!d-gi-z]",
            "abch",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "abc[!fgijkl]",
            "abch",
            PathMatch::empty()
        ));
        assert!(!archive_pathmatch(
            "abc[!fghijkl]",
            "abch",
            PathMatch::empty()
        ));

        /* Backslash quotes next character. */
        assert!(!archive_pathmatch(
            "abc\\[def]",
            "abc\\d",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "abc\\[def]",
            "abc[def]",
            PathMatch::empty()
        ));
        assert!(!archive_pathmatch(
            "abc\\\\[def]",
            "abc[def]",
            PathMatch::empty()
        ));
        assert!(!archive_pathmatch(
            "abc\\\\[def]",
            "abc\\[def]",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "abc\\\\[def]",
            "abc\\d",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch("abcd\\", "abcd\\", PathMatch::empty()));
        assert!(!archive_pathmatch("abcd\\", "abcd\\[", PathMatch::empty()));
        assert!(!archive_pathmatch("abcd\\", "abcde", PathMatch::empty()));
        assert!(!archive_pathmatch("abcd\\[", "abcd\\", PathMatch::empty()));

        /*
         * Because '.' and '/' have special meanings, we can
         * identify many equivalent paths even if they're expressed
         * differently.  (But quoting a character with '\\' suppresses
         * special meanings!)
         */
        assert!(!archive_pathmatch("a/b/", "a/bc", PathMatch::empty()));
        assert!(archive_pathmatch("a/./b", "a/b", PathMatch::empty()));
        assert!(!archive_pathmatch("a\\/./b", "a/b", PathMatch::empty()));
        assert!(!archive_pathmatch("a/\\./b", "a/b", PathMatch::empty()));
        assert!(!archive_pathmatch("a/.\\/b", "a/b", PathMatch::empty()));
        assert!(!archive_pathmatch("a\\/\\.\\/b", "a/b", PathMatch::empty()));
        assert!(archive_pathmatch(
            "./abc/./def/",
            "abc/def/",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "abc/def",
            "./././abc/./def",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "abc/def/././//",
            "./././abc/./def/",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            ".////abc/.//def",
            "./././abc/./def",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "./abc?def/",
            "abc/def/",
            PathMatch::empty()
        ));
        // failure("\"?./\" is not the same as \"/./\"");
        assert!(!archive_pathmatch(
            "./abc?./def/",
            "abc/def/",
            PathMatch::empty()
        ));
        // failure("Trailing '/' should match no trailing '/'");
        assert!(archive_pathmatch(
            "./abc/./def/",
            "abc/def",
            PathMatch::empty()
        ));
        // failure("Trailing '/./' is still the same directory.");
        assert!(archive_pathmatch(
            "./abc/./def/./",
            "abc/def",
            PathMatch::empty()
        ));
        // failure("Trailing '/.' is still the same directory.");
        assert!(archive_pathmatch(
            "./abc/./def/.",
            "abc/def",
            PathMatch::empty()
        ));
        assert!(archive_pathmatch(
            "./abc/./def",
            "abc/def/",
            PathMatch::empty()
        ));
        // failure("Trailing '/./' is still the same directory.");
        assert!(archive_pathmatch(
            "./abc/./def",
            "abc/def/./",
            PathMatch::empty()
        ));
        // failure("Trailing '/.' is still the same directory.");
        assert!(archive_pathmatch(
            "./abc*/./def",
            "abc/def/.",
            PathMatch::empty()
        ));
    }

    #[test]
    fn archive_path_match_no_anchor_start() {
        /* Matches not anchored at beginning. */
        assert!(!archive_pathmatch(
            "bcd",
            "abcd",
            PathMatch::NO_ANCHOR_START
        ));
        assert!(archive_pathmatch(
            "abcd",
            "abcd",
            PathMatch::NO_ANCHOR_START
        ));
        assert!(!archive_pathmatch(
            "^bcd",
            "abcd",
            PathMatch::NO_ANCHOR_START
        ));
        assert!(archive_pathmatch(
            "b/c/d",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START
        ));
        assert!(!archive_pathmatch(
            "^b/c/d",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START
        ));
        assert!(!archive_pathmatch(
            "/b/c/d",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START
        ));
        assert!(!archive_pathmatch(
            "a/b/c",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START
        ));
        assert!(archive_pathmatch(
            "a/b/c/d",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START
        ));
        assert!(!archive_pathmatch(
            "b/c",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START
        ));
        assert!(!archive_pathmatch(
            "^b/c",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START
        ));

        assert!(archive_pathmatch(
            "b/c/d",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START
        ));
        assert!(archive_pathmatch(
            "b/c/d",
            "/a/b/c/d",
            PathMatch::NO_ANCHOR_START
        ));

        /* Matches not anchored at end. */
        assert!(!archive_pathmatch("bcd", "abcd", PathMatch::NO_ANCHOR_END));
        assert!(archive_pathmatch("abcd", "abcd", PathMatch::NO_ANCHOR_END));
        assert!(archive_pathmatch("abcd", "abcd/", PathMatch::NO_ANCHOR_END));
        assert!(archive_pathmatch(
            "abcd",
            "abcd/.",
            PathMatch::NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch("abc", "abcd", PathMatch::NO_ANCHOR_END));
        assert!(archive_pathmatch(
            "a/b/c",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "a/b/c$",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "a/b/c$",
            "a/b/c",
            PathMatch::NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "a/b/c$",
            "a/b/c/",
            PathMatch::NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "a/b/c/",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "a/b/c/$",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "a/b/c/$",
            "a/b/c/",
            PathMatch::NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "a/b/c/$",
            "a/b/c",
            PathMatch::NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "b/c",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_END
        ));

        /* Matches not anchored at either end. */
        assert!(archive_pathmatch(
            "b/c",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "/b/c",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "/a/b/c",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "/a/b/c",
            "/a/b/c/d",
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "/a/b/c$",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "/a/b/c/d$",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "/a/b/c/d$",
            "/a/b/c/d/e",
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "/a/b/c/d$",
            "/a/b/c/d",
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "^a/b/c",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "^a/b/c$",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "a/b/c$",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "b/c/d$",
            "a/b/c/d",
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END
        ));
    }

    #[test]
    fn bsd_glob_all_matched_empty() {
        let m = BsdGlobMatcher::new(std::iter::empty::<&str>());
        assert!(m.all_matched());
    }

    #[test]
    fn mark_satisfied_idempotent() {
        let mut m = BsdGlobMatcher::new(["a.txt"]);
        m.mark_satisfied("a.txt");
        m.mark_satisfied("a.txt");
        assert!(m.all_matched());
    }

    #[test]
    fn mark_satisfied_nonmatching_is_noop() {
        let mut m = BsdGlobMatcher::new(["a.txt"]);
        m.mark_satisfied("b.txt");
        assert!(!m.all_matched());
    }

    #[test]
    fn bsd_glob_matcher_no_recursive_exact_match() {
        let mut m = BsdGlobMatcher::new(["dir/file.txt"]).with_no_recursive(true);
        assert!(m.matches("dir/file.txt"));
        assert!(m.all_matched());
    }

    #[test]
    fn bsd_glob_matcher_no_recursive_blocks_prefix() {
        let mut m = BsdGlobMatcher::new(["dir"]).with_no_recursive(true);
        // With no_recursive, pattern "dir" should NOT match "dir/file.txt"
        assert!(!m.matches("dir/file.txt"));
        // But should still match exact "dir"
        assert!(m.matches("dir"));
        assert!(m.all_matched());
    }

    #[test]
    fn bsd_glob_matcher_recursive_allows_prefix() {
        let mut m = BsdGlobMatcher::new(["dir"]).with_no_recursive(false);
        // With recursive (default), pattern "dir" SHOULD match "dir/file.txt"
        assert!(m.matches("dir/file.txt"));
        assert!(m.all_matched());
    }

    #[test]
    fn bsd_glob_matcher_glob_meta_ignores_no_recursive() {
        let mut m = BsdGlobMatcher::new(["dir/*.txt"]).with_no_recursive(true);
        // Patterns with glob meta always use match_inclusion, ignoring prefix logic
        assert!(m.matches("dir/file.txt"));
        assert!(m.all_matched());
    }

    #[test]
    fn bsd_glob_matcher_trailing_slash_normalization() {
        let mut m = BsdGlobMatcher::new(["dir/"]).with_no_recursive(false);
        // "dir/" should match "dir/file.txt" (trailing slash stripped)
        assert!(m.matches("dir/file.txt"));
        assert!(m.all_matched());
    }

    #[test]
    fn bsd_glob_matcher_multiple_patterns() {
        let mut m = BsdGlobMatcher::new(["*.txt", "*.rs"]);
        assert!(m.matches("file.txt"));
        assert!(!m.all_matched()); // *.rs not matched yet
        assert!(m.matches("main.rs"));
        assert!(m.all_matched());
    }

    #[test]
    fn bsd_glob_matcher_matches_any_pattern_no_side_effect() {
        let m = BsdGlobMatcher::new(["a.txt", "b.txt"]);
        // matches_any_pattern should not modify matched state
        assert!(m.matches_any_pattern("a.txt"));
        assert!(!m.all_matched());
    }

    #[test]
    fn bsd_glob_matcher_mark_satisfied_multiple_patterns() {
        let mut m = BsdGlobMatcher::new(["dir", "file.txt"]);
        m.mark_satisfied("dir/sub/file.txt");
        // Should mark "dir" as matched (prefix match)
        assert!(!m.all_matched()); // file.txt not matched
    }

    #[test]
    fn has_glob_meta_true() {
        assert!(has_glob_meta("*.txt"));
        assert!(has_glob_meta("file?.txt"));
        assert!(has_glob_meta("[abc]"));
        assert!(has_glob_meta("{a,b}"));
    }

    #[test]
    fn has_glob_meta_false() {
        assert!(!has_glob_meta("file.txt"));
        assert!(!has_glob_meta("dir/file.txt"));
        assert!(!has_glob_meta(""));
    }

    #[test]
    fn prefix_match_exact_not_prefix() {
        // Exact match is handled elsewhere, prefix_match only for "pattern/..."
        assert!(!prefix_match("dir", "dir"));
    }

    #[test]
    fn prefix_match_true_cases() {
        assert!(prefix_match("dir", "dir/file.txt"));
        assert!(prefix_match("dir/", "dir/file.txt"));
        assert!(prefix_match("a/b", "a/b/c/d"));
    }

    #[test]
    fn prefix_match_false_cases() {
        assert!(!prefix_match("dir", "directory"));
        assert!(!prefix_match("dir", "dir2/file.txt"));
        assert!(!prefix_match("dir", "other"));
    }

    #[test]
    fn bsd_glob_pattern_match_inclusion_exact() {
        let pat = BsdGlobPattern::new("file.txt");
        assert!(pat.match_inclusion("file.txt"));
    }

    #[test]
    fn bsd_glob_pattern_match_inclusion_wildcard() {
        let pat = BsdGlobPattern::new("*.txt");
        assert!(pat.match_inclusion("file.txt"));
        assert!(pat.match_inclusion("readme.txt"));
        assert!(!pat.match_inclusion("file.rs"));
    }

    #[test]
    fn bsd_glob_pattern_match_exclusion_substring() {
        let pat = BsdGlobPattern::new("test");
        // Exclusion uses NO_ANCHOR_START | NO_ANCHOR_END
        assert!(pat.match_exclusion("test"));
        assert!(pat.match_exclusion("file_test"));
        assert!(pat.match_exclusion("test_file"));
        assert!(pat.match_exclusion("prefix_test_suffix"));
    }

    #[test]
    fn bsd_glob_patterns_empty_matches_all() {
        let pats = BsdGlobPatterns::new(std::iter::empty::<&str>());
        assert!(pats.matches_inclusion("any.txt"));
        assert!(pats.matches_inclusion("path/to/file"));
    }

    #[test]
    fn bsd_glob_patterns_inclusion() {
        let pats = BsdGlobPatterns::new(["*.txt", "*.rs"]);
        assert!(pats.matches_inclusion("file.txt"));
        assert!(pats.matches_inclusion("main.rs"));
        assert!(!pats.matches_inclusion("file.md"));
    }

    #[test]
    fn bsd_glob_patterns_exclusion() {
        let pats = BsdGlobPatterns::new(["test", "tmp"]);
        assert!(pats.matches_exclusion("test_file"));
        assert!(pats.matches_exclusion("file_tmp"));
        assert!(!pats.matches_exclusion("other"));
    }

    #[test]
    fn ensure_all_matched_success() {
        let mut m = BsdGlobMatcher::new(["a.txt"]);
        m.mark_satisfied("a.txt");
        assert!(m.ensure_all_matched().is_ok());
    }

    #[test]
    fn ensure_all_matched_failure() {
        let m = BsdGlobMatcher::new(["nonexistent.txt"]);
        let result = m.ensure_all_matched();
        assert!(result.is_err());
    }

    #[test]
    fn glob_patterns_ensure_all_matched_failure() {
        let m = GlobPatterns::new(["nonexistent.txt"]).unwrap();
        let result = m.ensure_all_matched();
        assert!(result.is_err());
    }

    #[test]
    fn glob_patterns_matches_any_marks_matched() {
        let mut m = GlobPatterns::new(["*.txt"]).unwrap();
        assert!(m.matches_any("file.txt"));
        assert!(m.ensure_all_matched().is_ok());
    }
}