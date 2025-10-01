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
    pub(crate) fn is_empty(&self) -> bool {
        self.globs.is_empty()
    }

    #[inline]
    pub(crate) fn matches_any<P: AsRef<Path>>(&mut self, s: P) -> bool {
        let indices = self.globs.matches(s);
        for idx in indices.iter() {
            if let Some(found) = self.matched.get_mut(*idx) {
                *found = true;
            }
        }
        !indices.is_empty()
    }

    #[inline]
    pub(crate) fn unmatched_patterns(&self) -> Vec<&str> {
        let mut unmatched = Vec::new();
        for (idx, matched) in self.matched.iter().enumerate() {
            if !matched {
                unmatched.push(self.raw_patterns[idx]);
            }
        }
        unmatched
    }

    pub(crate) fn ensure_all_matched(&self) -> anyhow::Result<()> {
        let unmatched = self.unmatched_patterns();
        if !unmatched.is_empty() {
            for p in unmatched {
                log::error!("'{p}' not found in archive");
            }
            anyhow::bail!("from previous errors");
        }
        Ok(())
    }
}

/// BSD tar command like globs.
#[derive(Clone, Debug)]
pub(crate) struct BsdGlobPatterns(Vec<BsdGlobPattern>);

impl BsdGlobPatterns {
    #[inline]
    pub fn new(value: impl IntoIterator<Item = impl Into<String>>) -> Self {
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

impl<T: Into<String>> From<Vec<T>> for BsdGlobPatterns {
    #[inline]
    fn from(value: Vec<T>) -> Self {
        Self::new(value)
    }
}

/// BSD tar command like glob.
#[derive(Clone, Debug)]
pub(crate) struct BsdGlobPattern {
    pattern: String,
}

impl BsdGlobPattern {
    #[inline]
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
        }
    }

    #[inline]
    pub fn match_exclusion(&self, s: &str) -> bool {
        archive_pathmatch(
            &self.pattern,
            s,
            PathMatch::NO_ANCHOR_START | PathMatch::NO_ANCHOR_END,
        )
    }

    #[inline]
    pub fn match_inclusion(&self, s: &str) -> bool {
        archive_pathmatch(&self.pattern, s, PathMatch::NO_ANCHOR_START)
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
    while (s.starts_with('/')) || (s.starts_with("./")) || (s == ".") {
        let mut chars = s.chars();
        let _ = chars.next();
        s = chars.as_str();
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
        while let Some(_p) = p.strip_prefix('/') {
            p = _p;
        }
        while let Some(_s) = s.strip_prefix('/') {
            s = _s;
        }
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
    if s.starts_with("./") {
        s = pm_slashskip(&s[1..]);
    }
    if p.starts_with("./") {
        p = pm_slashskip(&p[1..]);
    }

    while let Some(c) = str_peek(p) {
        match c {
            '?' => {
                /* ? always succeeds, unless we hit end of 's' */
                if s.is_empty() {
                    return false;
                }
                p = str_skip_n(p, 1);
                s = str_skip_n(s, 1);
            }
            '*' => {
                /* "*" == "**" == "***" ... */
                while let Some(_p) = p.strip_prefix('*') {
                    p = _p;
                }
                /* Trailing '*' always succeeds. */
                if p.is_empty() {
                    return true;
                }
                while str_peek(s).is_some() {
                    if archive_pathmatch(p, s, flags) {
                        return true;
                    }
                    s = str_skip_n(s, 1);
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
                    if str_peek(s).is_some_and(|c| !pm_list(l, c, flags)) {
                        return false;
                    }
                    p = r;
                    s = str_skip_n(s, 1);
                } else {
                    /* No final ']', so just match '['. */
                    if str_peek(p) != str_peek(s) {
                        return false;
                    }
                    p = str_skip_n(p, 1);
                    s = str_skip_n(s, 1);
                }
            }
            '\\' => {
                /* Trailing '\\' matches itself. */
                if p.len() == 1 {
                    if str_peek(s).is_some_and(|c| c != '\\') {
                        return false;
                    }
                } else {
                    p = str_skip_n(p, 1);
                    if str_peek(p) != str_peek(s) {
                        return false;
                    }
                }
                p = str_skip_n(p, 1);
                s = str_skip_n(s, 1);
            }
            '/' => {
                if str_peek(s).is_some_and(|c| c != '/') {
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
                if str_peek(p) != str_peek(s) {
                    return false;
                }
                p = str_skip_n(p, 1);
                s = str_skip_n(s, 1);
            }
            _ => {
                if str_peek(p) != str_peek(s) {
                    return false;
                }
                p = str_skip_n(p, 1);
                s = str_skip_n(s, 1);
            }
        }
    }
    if str_peek(s) == Some('/') {
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

fn str_skip_n(s: &str, n: usize) -> &str {
    let mut _s = s.chars();
    for _ in 0..n {
        let _ = _s.next();
    }
    _s.as_str()
}

fn str_peek(s: &str) -> Option<char> {
    s.chars().next()
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
}
