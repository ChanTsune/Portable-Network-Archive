use regex::{Regex, RegexBuilder};
use std::{borrow::Cow, str::FromStr};

#[derive(thiserror::Error, Clone, Debug, PartialEq)]
pub(crate) enum TransformRuleError {
    #[error("Invalid transform rule format, transform rule must be starts with 's'")]
    StartsWithMustBeS,
    #[error("Invalid transform rule format")]
    InvalidFormat,
    #[error("Invalid flag: {0}")]
    InvalidFlag(char),
    #[error(transparent)]
    InvalidPattern(#[from] regex::Error),
}

#[derive(Clone, Debug)]
pub(crate) struct TransformRule {
    pattern: Regex,
    replacement: String,
    global: bool,
    #[allow(unused)]
    extended: bool,
    match_number: Option<usize>,
    apply_to_hardlinks: bool,
    apply_to_symlinks: bool,
    apply_to_regular_files: bool,
}

impl FromStr for TransformRule {
    type Err = TransformRuleError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl TransformRule {
    fn new(expression: &str) -> Result<Self, TransformRuleError> {
        let expression = if let Some(expression) = expression.strip_prefix('s') {
            expression
        } else {
            return Err(TransformRuleError::StartsWithMustBeS);
        };
        let mut chars = expression.chars();
        let delimiter = chars.next().ok_or(TransformRuleError::InvalidFormat)?;
        let mut parts = chars.as_str().split(delimiter);
        let pattern = parts.next().ok_or(TransformRuleError::InvalidFormat)?;
        let replacement = parts
            .next()
            .ok_or(TransformRuleError::InvalidFormat)?
            .to_string();
        let flags = parts.next().ok_or(TransformRuleError::InvalidFormat)?;

        let mut global = false;
        let mut case_insensitive = false;
        let mut extended = false;
        let mut match_number = None;
        let mut apply_to_hardlinks = true;
        let mut apply_to_symlinks = true;
        let mut apply_to_regular_files = true;

        for flag in flags.chars() {
            match flag {
                'g' => global = true,
                'i' => case_insensitive = true,
                'x' => extended = true,
                's' => apply_to_symlinks = true,
                'S' => apply_to_symlinks = false,
                'h' => apply_to_hardlinks = true,
                'H' => apply_to_hardlinks = false,
                'r' => apply_to_regular_files = true,
                'R' => apply_to_regular_files = false,
                f if f.is_ascii_digit() => match_number = f.to_digit(10).map(|it| it as usize),
                f => return Err(TransformRuleError::InvalidFlag(f)),
            }
        }

        let pattern = RegexBuilder::new(pattern)
            .case_insensitive(case_insensitive)
            .build()?;

        Ok(Self {
            pattern,
            replacement,
            global,
            extended,
            match_number,
            apply_to_hardlinks,
            apply_to_symlinks,
            apply_to_regular_files,
        })
    }

    fn apply<'a>(
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

        if let Some(n) = self.match_number {
            let mut count = 0;
            return Some(self.pattern.replace_all(input, |caps: &regex::Captures| {
                count += 1;
                if count == n {
                    self.replacement.clone()
                } else {
                    caps.get(0).unwrap().as_str().to_string()
                }
            }));
        }
        let result = if self.global {
            self.pattern.replace_all(input, self.replacement.as_str())
        } else {
            self.pattern.replace(input, self.replacement.as_str())
        };
        Some(result)
    }
}

fn apply_transforms(
    input: impl Into<String>,
    expressions: &[TransformRule],
    is_symlink: bool,
    is_hardlink: bool,
) -> String {
    let mut output = input.into();
    for rule in expressions {
        if let Some(applied) = rule.apply(&output, is_symlink, is_hardlink) {
            output = applied.into_owned();
        }
    }
    output
}

#[derive(Clone, Debug)]
pub(crate) struct TransformRules(Vec<TransformRule>);

impl TransformRules {
    #[inline]
    pub(crate) const fn new(rules: Vec<TransformRule>) -> TransformRules {
        Self(rules)
    }

    #[inline]
    pub(crate) fn apply(
        &self,
        input: impl Into<String>,
        is_symlink: bool,
        is_hardlink: bool,
    ) -> String {
        apply_transforms(input, &self.0, is_symlink, is_hardlink)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_replacement() {
        let input = "usr/bin";
        let expressions = TransformRule::from_str("s,usr/,usr/local/,").unwrap();
        let transformed = expressions.apply(input, false, false).unwrap();
        assert_eq!(transformed, "usr/local/bin");
    }

    #[test]
    fn strip_two_leading_directory_components() {
        let input = "usr/local/bin";
        let expressions = TransformRule::from_str("s,/*[^/]*/[^/]*/,,").unwrap();
        let transformed = expressions.apply(input, false, false).unwrap();
        assert_eq!(transformed, "bin");
    }

    #[test]
    fn case_insensitive() {
        let input = "UsR/local/bin";
        let expressions = TransformRule::from_str("s,usr,var,i").unwrap();
        let transformed = expressions.apply(input, false, false).unwrap();
        assert_eq!(transformed, "var/local/bin");
    }
    // TODO: Support this use case
    // #[test]
    // fn convert_each_file_name_to_lower_case() {
    //     let input = "UsR/locAl/Bin";
    //     let expressions = TransformRule::from_str("s/.*/\\L&/").unwrap();
    //     let transformed = expressions.apply(input, false, false).unwrap();
    //     assert_eq!(transformed, "usr/local/bin");
    // }

    #[test]
    fn prepend_prefix_to_each_file_name() {
        let input = "usr/local/bin";
        let expressions = TransformRule::from_str("s,^,prefix/,").unwrap();
        let transformed = expressions.apply(input, false, false).unwrap();
        assert_eq!(transformed, "prefix/usr/local/bin");
    }

    #[test]
    fn match_number() {
        let input = "foo foo foo";
        let expressions = TransformRule::from_str("s,foo,bar,2").unwrap();
        let transformed = expressions.apply(input, false, false).unwrap();
        assert_eq!(transformed, "foo bar foo");
    }
}
