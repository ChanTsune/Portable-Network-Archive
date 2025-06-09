use regex::{Captures, Regex, Replacer};
use std::{borrow::Cow, fmt::Debug, str::FromStr};

#[derive(thiserror::Error, Clone, Debug, PartialEq)]
pub(crate) enum SubstitutionError {
    #[error("Empty substitution rule")]
    Empty,
    #[error("Invalid substitution rule format")]
    InvalidFormat,
    #[error("Invalid flag: {0}")]
    InvalidFlag(char),
    #[error(transparent)]
    InvalidPattern(#[from] regex::Error),
}

#[derive(Clone, Debug)]
struct SubstitutionReplacer(String);

impl Replacer for SubstitutionReplacer {
    #[inline]
    fn replace_append(&mut self, caps: &Captures<'_>, result: &mut String) {
        let replacement = self.0.as_str();
        let mut chars = replacement.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(next_ch) = chars.peek() {
                    if let Some(group_index) = next_ch.to_digit(10) {
                        let group_index = group_index as usize;
                        if group_index < caps.len() {
                            result.push_str(&caps[group_index]);
                        }
                        chars.next();
                    } else {
                        result.push(ch);
                    }
                } else {
                    result.push(ch);
                }
            } else if ch == '~' {
                result.push_str(&caps[0]);
            } else {
                result.push(ch);
            }
        }
    }

    #[inline]
    fn no_expansion(&mut self) -> Option<Cow<'_, str>> {
        if self.0.find('~').is_some() || self.0.find('\\').is_some() {
            None
        } else {
            Some(Cow::Borrowed(&self.0))
        }
    }
}

/// Struct representing a substitution rule.
#[derive(Clone, Debug)]
pub(crate) struct SubstitutionRule {
    pattern: Regex,
    replacement: SubstitutionReplacer,
    global: bool,
    print: bool,
    apply_to_hardlinks: bool,
    apply_to_symlinks: bool,
    apply_to_regular_files: bool,
}

impl FromStr for SubstitutionRule {
    type Err = SubstitutionError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl SubstitutionRule {
    /// Parses a substitution rule from a BSD tar-style argument string.
    pub fn parse(rule: &str) -> Result<Self, SubstitutionError> {
        let mut rule = rule.chars();
        let delimiter = rule.next().ok_or(SubstitutionError::Empty)?;
        let mut parts = rule.as_str().split(delimiter);
        let pattern = parts.next().ok_or(SubstitutionError::InvalidFormat)?;
        let replacement = parts.next().ok_or(SubstitutionError::InvalidFormat)?;
        let flags = parts.next().ok_or(SubstitutionError::InvalidFormat)?;

        let mut global = false;
        let mut print = false;
        let mut apply_to_hardlinks = true;
        let mut apply_to_symlinks = true;
        let mut apply_to_regular_files = true;

        for flag in flags.chars() {
            match flag {
                'g' | 'G' => global = true,
                'p' | 'P' => print = true,
                's' => apply_to_symlinks = true,
                'S' => apply_to_symlinks = false,
                'h' => apply_to_hardlinks = true,
                'H' => apply_to_hardlinks = false,
                'r' => apply_to_regular_files = true,
                'R' => apply_to_regular_files = false,
                f => return Err(SubstitutionError::InvalidFlag(f)),
            }
        }

        let regex = Regex::new(pattern)?;
        Ok(Self {
            pattern: regex,
            replacement: SubstitutionReplacer(replacement.into()),
            global,
            print,
            apply_to_hardlinks,
            apply_to_symlinks,
            apply_to_regular_files,
        })
    }

    /// Applies the substitution to the input string.
    pub fn apply<'a>(
        &self,
        input: &'a str,
        is_symlink: bool,
        is_hardlink: bool,
    ) -> Option<Cow<'a, str>> {
        if is_symlink && !self.apply_to_symlinks {
            return None;
        }
        if is_hardlink && !self.apply_to_hardlinks {
            return None;
        }
        if !is_symlink && !is_hardlink && !self.apply_to_regular_files {
            return None;
        }

        let result = if self.global {
            self.pattern.replace_all(input, self.replacement.clone())
        } else {
            self.pattern.replace(input, self.replacement.clone())
        };

        if self.print {
            eprintln!("{input} >> {result}");
        }

        Some(result)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SubstitutionRules(Vec<SubstitutionRule>);

impl SubstitutionRules {
    #[inline]
    pub(crate) fn new(rules: Vec<SubstitutionRule>) -> Self {
        Self(rules)
    }
    #[inline]
    pub(crate) fn apply(
        &self,
        name: impl Into<String>,
        is_symlink: bool,
        is_hardlink: bool,
    ) -> String {
        apply_substitutions(name, &self.0, is_symlink, is_hardlink)
    }
}

fn apply_substitutions(
    name: impl Into<String>,
    substitutions: &[SubstitutionRule],
    is_symlink: bool,
    is_hardlink: bool,
) -> String {
    let mut output = name.into();
    for rule in substitutions {
        if let Some(applied) = rule.apply(&output, is_symlink, is_hardlink) {
            output = applied.into_owned();
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_apply_single() {
        let substitution = SubstitutionRule::parse("/foo/bar/").unwrap();
        let input = "foo baz foo";
        let result = substitution.apply(input, false, false).unwrap();
        assert_eq!(result, "bar baz foo");
    }

    #[test]
    fn parse_and_apply_global() {
        let substitution = SubstitutionRule::parse("/foo/bar/g").unwrap();
        let input = "foo baz foo";
        let result = substitution.apply(input, false, false).unwrap();
        assert_eq!(result, "bar baz bar");
    }

    #[test]
    fn parse_and_apply_regular_files() {
        let substitution = SubstitutionRule::parse("/foo/abc/r").unwrap();
        let input = "foo baz foo";
        let result = substitution.apply(input, false, false).unwrap();
        assert_eq!(result, "abc baz foo");
    }

    #[test]
    fn parse_and_skip_regular_files() {
        let substitution = SubstitutionRule::parse("/foo/abc/R").unwrap();
        let input = "foo baz foo";
        let result = substitution.apply(input, false, false);
        assert!(result.is_none());
    }

    #[test]
    fn parse_and_apply_symlinks() {
        let substitution = SubstitutionRule::parse("/foo/bar/s").unwrap();
        let input = "foo baz foo";
        let result = substitution.apply(input, true, false).unwrap();
        assert_eq!(result, "bar baz foo");
    }

    #[test]
    fn parse_and_skip_symlinks() {
        let substitution = SubstitutionRule::parse("/foo/bar/S").unwrap();
        let input = "foo baz foo";
        let result = substitution.apply(input, true, false);
        assert!(result.is_none());
    }

    #[test]
    fn parse_and_apply_hardlinks() {
        let substitution = SubstitutionRule::parse("/foo/bar/h").unwrap();
        let input = "foo baz foo";
        let result = substitution.apply(input, false, true).unwrap();
        assert_eq!(result, "bar baz foo");
    }

    #[test]
    fn parse_and_skip_hardlinks() {
        let substitution = SubstitutionRule::parse("/foo/bar/H").unwrap();
        let input = "foo baz foo";
        let result = substitution.apply(input, false, true);
        assert!(result.is_none());
    }

    #[test]
    fn parse_and_notify() {
        let substitution = SubstitutionRule::parse("/foo/bar/p").unwrap();
        let input = "foo baz foo";
        let result = substitution.apply(input, false, false).unwrap();
        assert_eq!(result, "bar baz foo");
    }

    #[test]
    fn multiple_captures() {
        let substitution = SubstitutionRule::parse("/(foo)/\\1bar/g").unwrap();
        let input = "foo baz foo";
        let result = substitution.apply(input, false, false).unwrap();
        assert_eq!(result, "foobar baz foobar");

        let substitution = SubstitutionRule::parse("/(foo)/\\1bar/").unwrap();
        let input = "foo baz foo";
        let result = substitution.apply(input, false, false).unwrap();
        assert_eq!(result, "foobar baz foo");
    }

    #[test]
    fn tilde_replacement() {
        let substitution = SubstitutionRule::parse("/foo/~bar~/g").unwrap();
        let input = "foo baz foo";
        let result = substitution.apply(input, false, false).unwrap();
        assert_eq!(result, "foobarfoo baz foobarfoo");
    }
}
