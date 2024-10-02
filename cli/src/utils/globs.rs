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
    fn glob_any() {
        let globs = GlobPatterns::new(vec!["path/**"]).unwrap();
        assert!(globs.matches_any("path/foo.pna"));
    }
}
