use std::path::Path;

#[derive(Clone, Debug)]
pub(crate) struct GlobPatterns(globset::GlobSet);

impl GlobPatterns {
    #[inline]
    pub(crate) fn new<I: IntoIterator<Item = S>, S: AsRef<str>>(
        patterns: I,
    ) -> Result<Self, globset::Error> {
        let mut builder = globset::GlobSet::builder();
        for pattern in patterns {
            builder.add(globset::Glob::new(pattern.as_ref())?);
        }
        Ok(Self(builder.build()?))
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[inline]
    pub(crate) fn matches_any<P: AsRef<Path>>(&self, s: P) -> bool {
        self.0.is_match(s)
    }
}

impl TryFrom<Vec<globset::Glob>> for GlobPatterns {
    type Error = globset::Error;

    #[inline]
    fn try_from(patterns: Vec<globset::Glob>) -> Result<Self, Self::Error> {
        let mut builder = globset::GlobSet::builder();
        for pattern in patterns {
            builder.add(pattern);
        }
        Ok(Self(builder.build()?))
    }
}
#[derive(Clone, Debug)]
pub(crate) struct ExcludeGlobPatterns(Vec<ExcludeGlobPattern>);

impl ExcludeGlobPatterns {
    #[inline]
    pub fn new(
        value: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, core::convert::Infallible> {
        Ok(Self(
            value
                .into_iter()
                .map(|it| ExcludeGlobPattern::WildCard(it.into()))
                .collect(),
        ))
    }

    #[inline]
    pub fn matches_any(&self, s: impl AsRef<str>) -> bool {
        self.0.iter().any(|it| it.is_match(s.as_ref()))
    }
}

impl<T: Into<String>> From<Vec<T>> for ExcludeGlobPatterns {
    #[inline]
    fn from(value: Vec<T>) -> Self {
        Self::new(value).unwrap()
    }
}

#[derive(Clone, Debug)]
pub(crate) enum ExcludeGlobPattern {
    WildCard(String),
}

impl ExcludeGlobPattern {
    #[inline]
    pub fn is_match(&self, s: &str) -> bool {
        match self {
            Self::WildCard(pattern) => archive_pathmatch(
                pattern,
                s,
                PATHMATCH_NO_ANCHOR_START | PATHMATCH_NO_ANCHOR_END,
            ),
        }
    }
}

/* Don't anchor at beginning unless the pattern starts with "^" */
const PATHMATCH_NO_ANCHOR_START: usize = 1;
/* Don't anchor at end unless the pattern ends with "$" */
const PATHMATCH_NO_ANCHOR_END: usize = 2;

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
fn pm_list(mut class: &str, c: char, _flags: usize) -> bool {
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
                if range_start.is_none() || (chars.clone().count() == 0) {
                    if p == c {
                        return r#match;
                    }
                } else {
                    let mut range_end = chars.next().unwrap();
                    if range_end == '\\' {
                        range_end = chars.next().unwrap();
                    }
                    if (range_start.unwrap() <= c) && (c <= range_end) {
                        return r#match;
                    }
                }
            }
            '\\' => {
                let p = chars.next().unwrap();
                if p == c {
                    return r#match;
                }
                next_range_start = Some(p); /* Possible start of range. */
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
fn archive_pathmatch(mut p: &str, mut s: &str, mut flags: usize) -> bool {
    /* Empty pattern only matches the empty string. */
    if p.is_empty() {
        return s.is_empty();
    }

    /* Leading '^' anchors the start of the pattern. */
    if let Some(_p) = p.strip_prefix('^') {
        flags &= !PATHMATCH_NO_ANCHOR_START;
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
    if (flags & PATHMATCH_NO_ANCHOR_START) != 0 {
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

fn pm(mut p: &str, mut s: &str, flags: usize) -> bool {
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
                    if !pm_list(l, str_peek(s).unwrap(), flags) {
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
                    if str_peek(s).unwrap() != '\\' {
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
                if !s.is_empty() && str_peek(s).unwrap() != '/' {
                    return false;
                }
                /* Note: pattern "/\./" won't match "/";
                 * pm_slashskip() correctly stops at backslash. */
                p = pm_slashskip(p);
                s = pm_slashskip(s);
                if p.is_empty() && (flags & PATHMATCH_NO_ANCHOR_END) != 0 {
                    return true;
                }
            }
            '$' => {
                /* '$' is special only at end of pattern and only
                 * if PATHMATCH_NO_ANCHOR_END is specified. */
                if p.len() == 1 && (flags & PATHMATCH_NO_ANCHOR_END) != 0 {
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
        if (flags & PATHMATCH_NO_ANCHOR_END) != 0 {
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
        let globs = GlobPatterns::try_from(Vec::new()).unwrap();
        assert!(!globs.matches_any("some"));
    }

    #[test]
    fn glob_suffix() {
        let globs = GlobPatterns::new(vec!["path/**"]).unwrap();
        assert!(globs.matches_any("path/foo.pna"));
    }

    #[test]
    fn glob_prefix() {
        let globs = GlobPatterns::new(vec!["**/foo.pna"]).unwrap();
        assert!(globs.matches_any("path/foo.pna"));
        assert!(globs.matches_any("path/path/foo.pna"));
    }

    #[test]
    fn glob_middle_component() {
        let globs = GlobPatterns::new(vec!["usr/**/bin"]).unwrap();
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
        assert!(archive_pathmatch("a/b/c", "a/b/c", 0));
        assert!(!archive_pathmatch("a/b/", "a/b/c", 0));
        assert!(!archive_pathmatch("a/b", "a/b/c", 0));
        assert!(!archive_pathmatch("a/b/c", "a/b/", 0));
        assert!(!archive_pathmatch("a/b/c", "a/b", 0));

        // /* Null string and non-empty pattern returns false. */
        // assert!(! archive_pathmatch("a/b/c", NULL, 0));
        // assert!(! archive_pathmatch_w(L"a/b/c", NULL, 0));

        /* Empty pattern only matches empty string. */
        assert!(archive_pathmatch("", "", 0));
        assert!(!archive_pathmatch("", "a", 0));
        assert!(archive_pathmatch("*", "", 0));
        assert!(archive_pathmatch("*", "a", 0));
        assert!(archive_pathmatch("*", "abcd", 0));
        /* SUSv2: * matches / */
        assert!(archive_pathmatch("*", "abcd/efgh/ijkl", 0));
        assert!(archive_pathmatch("abcd*efgh/ijkl", "abcd/efgh/ijkl", 0));
        assert!(archive_pathmatch("abcd***efgh/ijkl", "abcd/efgh/ijkl", 0));
        assert!(archive_pathmatch("abcd***/efgh/ijkl", "abcd/efgh/ijkl", 0));
        assert!(!archive_pathmatch("?", "", 0));
        // assert!(!archive_pathmatch("?", "\0", 0));
        assert!(archive_pathmatch("?", "a", 0));
        assert!(!archive_pathmatch("?", "ab", 0));
        assert!(archive_pathmatch("?", ".", 0));
        assert!(archive_pathmatch("?", "?", 0));
        assert!(archive_pathmatch("a", "a", 0));
        assert!(!archive_pathmatch("a", "ab", 0));
        assert!(!archive_pathmatch("a", "ab", 0));
        assert!(archive_pathmatch("a?c", "abc", 0));
        /* SUSv2: ? matches / */
        assert!(archive_pathmatch("a?c", "a/c", 0));
        assert!(archive_pathmatch("a?*c*", "a/c", 0));
        assert!(archive_pathmatch("*a*", "a/c", 0));
        assert!(archive_pathmatch("*a*", "/a/c", 0));
        assert!(archive_pathmatch("*a*", "defaaaaaaa", 0));
        assert!(!archive_pathmatch("a*", "defghi", 0));
        assert!(!archive_pathmatch("*a*", "defghi", 0));

        /* Character classes */
        assert!(archive_pathmatch("abc[def", "abc[def", 0));
        assert!(!archive_pathmatch("abc[def]", "abc[def", 0));
        assert!(!archive_pathmatch("abc[def", "abcd", 0));
        assert!(archive_pathmatch("abc[def]", "abcd", 0));
        assert!(archive_pathmatch("abc[def]", "abce", 0));
        assert!(archive_pathmatch("abc[def]", "abcf", 0));
        assert!(!archive_pathmatch("abc[def]", "abcg", 0));
        assert!(archive_pathmatch("abc[d*f]", "abcd", 0));
        assert!(archive_pathmatch("abc[d*f]", "abc*", 0));
        assert!(!archive_pathmatch("abc[d*f]", "abcdefghi", 0));
        assert!(!archive_pathmatch("abc[d*", "abcdefghi", 0));
        assert!(archive_pathmatch("abc[d*", "abc[defghi", 0));
        assert!(archive_pathmatch("abc[d-f]", "abcd", 0));
        assert!(archive_pathmatch("abc[d-f]", "abce", 0));
        assert!(archive_pathmatch("abc[d-f]", "abcf", 0));
        assert!(!archive_pathmatch("abc[d-f]", "abcg", 0));
        assert!(!archive_pathmatch("abc[d-fh-k]", "abca", 0));
        assert!(archive_pathmatch("abc[d-fh-k]", "abcd", 0));
        assert!(archive_pathmatch("abc[d-fh-k]", "abce", 0));
        assert!(archive_pathmatch("abc[d-fh-k]", "abcf", 0));
        assert!(!archive_pathmatch("abc[d-fh-k]", "abcg", 0));
        assert!(archive_pathmatch("abc[d-fh-k]", "abch", 0));
        assert!(archive_pathmatch("abc[d-fh-k]", "abci", 0));
        assert!(archive_pathmatch("abc[d-fh-k]", "abcj", 0));
        assert!(archive_pathmatch("abc[d-fh-k]", "abck", 0));
        assert!(!archive_pathmatch("abc[d-fh-k]", "abcl", 0));
        assert!(!archive_pathmatch("abc[d-fh-k]", "abc-", 0));

        /* [] matches nothing, [!] is the same as ? */
        assert!(!archive_pathmatch("abc[]efg", "abcdefg", 0));
        assert!(!archive_pathmatch("abc[]efg", "abcqefg", 0));
        assert!(!archive_pathmatch("abc[]efg", "abcefg", 0));
        assert!(archive_pathmatch("abc[!]efg", "abcdefg", 0));
        assert!(archive_pathmatch("abc[!]efg", "abcqefg", 0));
        assert!(!archive_pathmatch("abc[!]efg", "abcefg", 0));

        /* I assume: Trailing '-' is non-special. */
        assert!(!archive_pathmatch("abc[d-fh-]", "abcl", 0));
        assert!(archive_pathmatch("abc[d-fh-]", "abch", 0));
        assert!(archive_pathmatch("abc[d-fh-]", "abc-", 0));
        assert!(archive_pathmatch("abc[d-fh-]", "abc-", 0));

        /* ']' can be backslash-quoted within a character class. */
        assert!(archive_pathmatch("abc[\\]]", "abc]", 0));
        assert!(archive_pathmatch("abc[\\]d]", "abc]", 0));
        assert!(archive_pathmatch("abc[\\]d]", "abcd", 0));
        assert!(archive_pathmatch("abc[d\\]]", "abc]", 0));
        assert!(archive_pathmatch("abc[d\\]]", "abcd", 0));
        assert!(archive_pathmatch("abc[d]e]", "abcde]", 0));
        assert!(archive_pathmatch("abc[d\\]e]", "abc]", 0));
        assert!(!archive_pathmatch("abc[d\\]e]", "abcd]e", 0));
        assert!(!archive_pathmatch("abc[d]e]", "abc]", 0));

        /* backslash-quoted chars can appear as either end of a range. */
        assert!(archive_pathmatch("abc[\\d-f]gh", "abcegh", 0));
        assert!(!archive_pathmatch("abc[\\d-f]gh", "abcggh", 0));
        assert!(!archive_pathmatch("abc[\\d-f]gh", "abc\\gh", 0));
        assert!(archive_pathmatch("abc[d-\\f]gh", "abcegh", 0));
        assert!(archive_pathmatch("abc[\\d-\\f]gh", "abcegh", 0));
        assert!(archive_pathmatch("abc[\\d-\\f]gh", "abcegh", 0));
        /* backslash-quoted '-' isn't special. */
        assert!(!archive_pathmatch("abc[d\\-f]gh", "abcegh", 0));
        assert!(archive_pathmatch("abc[d\\-f]gh", "abc-gh", 0));

        /* Leading '!' negates a character class. */
        assert!(!archive_pathmatch("abc[!d]", "abcd", 0));
        assert!(archive_pathmatch("abc[!d]", "abce", 0));
        assert!(archive_pathmatch("abc[!d]", "abcc", 0));
        assert!(!archive_pathmatch("abc[!d-z]", "abcq", 0));
        assert!(archive_pathmatch("abc[!d-gi-z]", "abch", 0));
        assert!(archive_pathmatch("abc[!fgijkl]", "abch", 0));
        assert!(!archive_pathmatch("abc[!fghijkl]", "abch", 0));

        /* Backslash quotes next character. */
        assert!(!archive_pathmatch("abc\\[def]", "abc\\d", 0));
        assert!(archive_pathmatch("abc\\[def]", "abc[def]", 0));
        assert!(!archive_pathmatch("abc\\\\[def]", "abc[def]", 0));
        assert!(!archive_pathmatch("abc\\\\[def]", "abc\\[def]", 0));
        assert!(archive_pathmatch("abc\\\\[def]", "abc\\d", 0));
        assert!(archive_pathmatch("abcd\\", "abcd\\", 0));
        assert!(!archive_pathmatch("abcd\\", "abcd\\[", 0));
        assert!(!archive_pathmatch("abcd\\", "abcde", 0));
        assert!(!archive_pathmatch("abcd\\[", "abcd\\", 0));

        /*
         * Because '.' and '/' have special meanings, we can
         * identify many equivalent paths even if they're expressed
         * differently.  (But quoting a character with '\\' suppresses
         * special meanings!)
         */
        assert!(!archive_pathmatch("a/b/", "a/bc", 0));
        assert!(archive_pathmatch("a/./b", "a/b", 0));
        assert!(!archive_pathmatch("a\\/./b", "a/b", 0));
        assert!(!archive_pathmatch("a/\\./b", "a/b", 0));
        assert!(!archive_pathmatch("a/.\\/b", "a/b", 0));
        assert!(!archive_pathmatch("a\\/\\.\\/b", "a/b", 0));
        assert!(archive_pathmatch("./abc/./def/", "abc/def/", 0));
        assert!(archive_pathmatch("abc/def", "./././abc/./def", 0));
        assert!(archive_pathmatch("abc/def/././//", "./././abc/./def/", 0));
        assert!(archive_pathmatch(".////abc/.//def", "./././abc/./def", 0));
        assert!(archive_pathmatch("./abc?def/", "abc/def/", 0));
        // failure("\"?./\" is not the same as \"/./\"");
        assert!(!archive_pathmatch("./abc?./def/", "abc/def/", 0));
        // failure("Trailing '/' should match no trailing '/'");
        assert!(archive_pathmatch("./abc/./def/", "abc/def", 0));
        // failure("Trailing '/./' is still the same directory.");
        assert!(archive_pathmatch("./abc/./def/./", "abc/def", 0));
        // failure("Trailing '/.' is still the same directory.");
        assert!(archive_pathmatch("./abc/./def/.", "abc/def", 0));
        assert!(archive_pathmatch("./abc/./def", "abc/def/", 0));
        // failure("Trailing '/./' is still the same directory.");
        assert!(archive_pathmatch("./abc/./def", "abc/def/./", 0));
        // failure("Trailing '/.' is still the same directory.");
        assert!(archive_pathmatch("./abc*/./def", "abc/def/.", 0));
    }

    #[test]
    fn archive_path_match_no_anchor_start() {
        /* Matches not anchored at beginning. */
        assert!(!archive_pathmatch("bcd", "abcd", PATHMATCH_NO_ANCHOR_START));
        assert!(archive_pathmatch("abcd", "abcd", PATHMATCH_NO_ANCHOR_START));
        assert!(!archive_pathmatch(
            "^bcd",
            "abcd",
            PATHMATCH_NO_ANCHOR_START
        ));
        assert!(archive_pathmatch(
            "b/c/d",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START
        ));
        assert!(!archive_pathmatch(
            "^b/c/d",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START
        ));
        assert!(!archive_pathmatch(
            "/b/c/d",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START
        ));
        assert!(!archive_pathmatch(
            "a/b/c",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START
        ));
        assert!(archive_pathmatch(
            "a/b/c/d",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START
        ));
        assert!(!archive_pathmatch(
            "b/c",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START
        ));
        assert!(!archive_pathmatch(
            "^b/c",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START
        ));

        assert!(archive_pathmatch(
            "b/c/d",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START
        ));
        assert!(archive_pathmatch(
            "b/c/d",
            "/a/b/c/d",
            PATHMATCH_NO_ANCHOR_START
        ));

        /* Matches not anchored at end. */
        assert!(!archive_pathmatch("bcd", "abcd", PATHMATCH_NO_ANCHOR_END));
        assert!(archive_pathmatch("abcd", "abcd", PATHMATCH_NO_ANCHOR_END));
        assert!(archive_pathmatch("abcd", "abcd/", PATHMATCH_NO_ANCHOR_END));
        assert!(archive_pathmatch("abcd", "abcd/.", PATHMATCH_NO_ANCHOR_END));
        assert!(!archive_pathmatch("abc", "abcd", PATHMATCH_NO_ANCHOR_END));
        assert!(archive_pathmatch(
            "a/b/c",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "a/b/c$",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "a/b/c$",
            "a/b/c",
            PATHMATCH_NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "a/b/c$",
            "a/b/c/",
            PATHMATCH_NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "a/b/c/",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "a/b/c/$",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "a/b/c/$",
            "a/b/c/",
            PATHMATCH_NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "a/b/c/$",
            "a/b/c",
            PATHMATCH_NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "b/c",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_END
        ));

        /* Matches not anchored at either end. */
        assert!(archive_pathmatch(
            "b/c",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START | PATHMATCH_NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "/b/c",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START | PATHMATCH_NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "/a/b/c",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START | PATHMATCH_NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "/a/b/c",
            "/a/b/c/d",
            PATHMATCH_NO_ANCHOR_START | PATHMATCH_NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "/a/b/c$",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START | PATHMATCH_NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "/a/b/c/d$",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START | PATHMATCH_NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "/a/b/c/d$",
            "/a/b/c/d/e",
            PATHMATCH_NO_ANCHOR_START | PATHMATCH_NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "/a/b/c/d$",
            "/a/b/c/d",
            PATHMATCH_NO_ANCHOR_START | PATHMATCH_NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "^a/b/c",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START | PATHMATCH_NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "^a/b/c$",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START | PATHMATCH_NO_ANCHOR_END
        ));
        assert!(!archive_pathmatch(
            "a/b/c$",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START | PATHMATCH_NO_ANCHOR_END
        ));
        assert!(archive_pathmatch(
            "b/c/d$",
            "a/b/c/d",
            PATHMATCH_NO_ANCHOR_START | PATHMATCH_NO_ANCHOR_END
        ));
    }
}
