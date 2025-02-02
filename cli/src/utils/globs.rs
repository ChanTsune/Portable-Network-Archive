use std::path::Path;

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

    #[inline]
    pub(crate) fn starts_with_matches_any<P: AsRef<Path>>(&self, s: P) -> bool {
        let p = s.as_ref();
        p.ancestors().any(|it| self.0.is_match(it))
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
    fn glob_starts_with() {
        let globs = GlobPatterns::new(vec!["usr"]).unwrap();
        assert!(globs.starts_with_matches_any("usr/local/bin"));
        assert!(globs.starts_with_matches_any("usr/share/bin"));
        assert!(!globs.starts_with_matches_any("etc/usr/bin"));
        let globs = GlobPatterns::new(vec!["usr/**"]).unwrap();
        assert!(globs.starts_with_matches_any("usr/local/bin"));
        assert!(globs.starts_with_matches_any("usr/share/bin"));
        assert!(!globs.starts_with_matches_any("etc/usr/bin"));
        let globs = GlobPatterns::new(vec!["**/bin"]).unwrap();
        assert!(globs.starts_with_matches_any("usr/local/bin"));
        assert!(globs.starts_with_matches_any("usr/share/bin"));
    }
}
