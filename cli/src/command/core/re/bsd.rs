use regex::{Captures, Regex};
use std::{fmt::Debug, str::FromStr};

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
    from_begin: bool,
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
        let mut from_begin = false;

        for flag in flags.chars() {
            match flag {
                'g' | 'G' => global = true,
                'p' | 'P' => print = true,
                'b' | 'B' => from_begin = true,
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
            from_begin,
        })
    }

    fn applies_to(&self, is_symlink: bool, is_hardlink: bool) -> bool {
        if is_symlink && !self.apply_to_symlinks {
            return false;
        }
        if is_hardlink && !self.apply_to_hardlinks {
            return false;
        }
        if !is_symlink && !is_hardlink && !self.apply_to_regular_files {
            return false;
        }
        true
    }

    fn append_replacement(&self, caps: &Captures<'_>, result: &mut String) {
        let replacement = self.replacement.0.as_str();
        let mut chars = replacement.chars();
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('~') => result.push('~'),
                    Some('\\') => result.push('\\'),
                    Some(c @ '1'..='9') => {
                        let group_index = (c as usize) - ('0' as usize);
                        if let Some(m) = caps.get(group_index) {
                            result.push_str(m.as_str());
                        }
                    }
                    Some(c) => {
                        result.push('\\');
                        result.push(c);
                    }
                    None => result.push('\\'),
                }
            } else if ch == '~' {
                result.push_str(&caps[0]);
            } else {
                result.push(ch);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SubstitutionRules(Vec<SubstitutionRule>);

impl SubstitutionRules {
    #[inline]
    pub(crate) const fn new(rules: Vec<SubstitutionRule>) -> Self {
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

/// Applies substitution rules using bsdtar-compatible position-tracking semantics.
///
/// Each rule operates on the remainder of the source string after previous rules
/// consumed their matched portions. The `from_begin` flag (`b`/`B`) resets the
/// source to the accumulated result before applying that rule.
fn apply_substitutions(
    name: impl Into<String>,
    substitutions: &[SubstitutionRule],
    is_symlink: bool,
    is_hardlink: bool,
) -> String {
    let original = name.into();
    let mut source = original.clone();
    let mut pos: usize = 0;
    let mut result = String::new();
    let mut got_match = false;
    let mut print_match = false;

    for rule in substitutions {
        if !rule.applies_to(is_symlink, is_hardlink) {
            continue;
        }

        // from_begin: reset matching position to accumulated result
        if rule.from_begin && got_match {
            result.push_str(&source[pos..]);
            source = std::mem::take(&mut result);
            pos = 0;
        }

        loop {
            let remaining = &source[pos..];
            let is_end = remaining.is_empty();

            let Some(captures) = rule.pattern.captures(remaining) else {
                break;
            };
            let m = captures.get(0).unwrap();

            got_match = true;
            print_match |= rule.print;

            // Append pre-match text
            result.push_str(&remaining[..m.start()]);

            // Append replacement
            rule.append_replacement(&captures, &mut result);

            // Advance past match (bsdtar checks rm_eo > 0, not rm_eo > rm_so)
            if m.end() > 0 {
                pos += m.end();
            } else if !is_end {
                // Zero-length match: copy one character and advance
                let advance = remaining.chars().next().map_or(0, |c| c.len_utf8());
                result.push_str(&remaining[..advance]);
                pos += advance;
            }

            if !rule.global || is_end {
                break;
            }
        }
    }

    if got_match {
        result.push_str(&source[pos..]);
        if print_match {
            eprintln!("{original} >> {result}");
        }
        result
    } else {
        source
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(specs: &[&str]) -> SubstitutionRules {
        SubstitutionRules::new(
            specs
                .iter()
                .map(|s| SubstitutionRule::parse(s).unwrap())
                .collect(),
        )
    }

    #[test]
    fn single_substitution() {
        assert_eq!(
            rules(&["/foo/bar/"]).apply("foo baz foo", false, false),
            "bar baz foo"
        );
    }

    #[test]
    fn global_substitution() {
        assert_eq!(
            rules(&["/foo/bar/g"]).apply("foo baz foo", false, false),
            "bar baz bar"
        );
    }

    #[test]
    fn parse_from_begin_flag() {
        let substitution = SubstitutionRule::parse("/ar/az/b").unwrap();
        assert!(substitution.from_begin);
    }

    #[test]
    fn multi_rule_position_tracking() {
        // bsdtar test_option_s test 4: -s /foo/bar/ -s }bar}baz}
        // After rule 1 matches, rule 2 sees only the remainder of the original
        // string (empty), so it cannot match — preventing double-substitution.
        let rules = SubstitutionRules::new(vec![
            SubstitutionRule::parse("/foo/bar/").unwrap(),
            SubstitutionRule::parse("}bar}baz}").unwrap(),
        ]);

        // "foo" matched by rule 1 → "bar"; rule 2 sees "" → no match
        assert_eq!(rules.apply("in/d1/foo", false, false), "in/d1/bar");
        // "foo" not in "bar" → rule 1 skips; "bar" matched by rule 2 → "baz"
        assert_eq!(rules.apply("in/d1/bar", false, false), "in/d1/baz");
    }

    #[test]
    fn multi_rule_name_swap() {
        // bsdtar test_option_s test 5: -s /foo/bar/ -s }bar}foo}
        let rules = SubstitutionRules::new(vec![
            SubstitutionRule::parse("/foo/bar/").unwrap(),
            SubstitutionRule::parse("}bar}foo}").unwrap(),
        ]);

        assert_eq!(rules.apply("in/d1/foo", false, false), "in/d1/bar");
        assert_eq!(rules.apply("in/d1/bar", false, false), "in/d1/foo");
    }

    #[test]
    fn multi_rule_with_from_begin_flag() {
        // bsdtar test_option_s 4b: -s /oo/ar/ -s }ar}az}b
        // Rule 2 has `b` (from_begin): resets position to accumulated result.
        let rules = SubstitutionRules::new(vec![
            SubstitutionRule::parse("/oo/ar/").unwrap(),
            SubstitutionRule::parse("}ar}az}b").unwrap(),
        ]);

        assert_eq!(rules.apply("in/d1/foo", false, false), "in/d1/faz");
        assert_eq!(rules.apply("in/d1/bar", false, false), "in/d1/baz");
    }

    #[test]
    fn multi_rule_three_with_from_begin_flag() {
        // bsdtar test_option_s 4bb: -s /oo/ar/ -s }ar}az}b -s :az:end:b
        let rules = SubstitutionRules::new(vec![
            SubstitutionRule::parse("/oo/ar/").unwrap(),
            SubstitutionRule::parse("}ar}az}b").unwrap(),
            SubstitutionRule::parse(":az:end:b").unwrap(),
        ]);

        assert_eq!(rules.apply("in/d1/foo", false, false), "in/d1/fend");
        assert_eq!(rules.apply("in/d1/bar", false, false), "in/d1/bend");
    }

    #[test]
    fn apply_to_regular_files() {
        assert_eq!(
            rules(&["/foo/abc/r"]).apply("foo baz foo", false, false),
            "abc baz foo"
        );
    }

    #[test]
    fn skip_regular_files() {
        assert_eq!(
            rules(&["/foo/abc/R"]).apply("foo baz foo", false, false),
            "foo baz foo"
        );
    }

    #[test]
    fn apply_to_symlinks() {
        assert_eq!(
            rules(&["/foo/bar/s"]).apply("foo baz foo", true, false),
            "bar baz foo"
        );
    }

    #[test]
    fn skip_symlinks() {
        assert_eq!(
            rules(&["/foo/bar/S"]).apply("foo baz foo", true, false),
            "foo baz foo"
        );
    }

    #[test]
    fn apply_to_hardlinks() {
        assert_eq!(
            rules(&["/foo/bar/h"]).apply("foo baz foo", false, true),
            "bar baz foo"
        );
    }

    #[test]
    fn skip_hardlinks() {
        assert_eq!(
            rules(&["/foo/bar/H"]).apply("foo baz foo", false, true),
            "foo baz foo"
        );
    }

    #[test]
    fn print_flag() {
        assert_eq!(
            rules(&["/foo/bar/p"]).apply("foo baz foo", false, false),
            "bar baz foo"
        );
    }

    #[test]
    fn backreference() {
        assert_eq!(
            rules(&["/(foo)/\\1bar/g"]).apply("foo baz foo", false, false),
            "foobar baz foobar"
        );
        assert_eq!(
            rules(&["/(foo)/\\1bar/"]).apply("foo baz foo", false, false),
            "foobar baz foo"
        );
    }

    #[test]
    fn tilde_replacement() {
        assert_eq!(
            rules(&["/foo/~bar~/g"]).apply("foo baz foo", false, false),
            "foobarfoo baz foobarfoo"
        );
    }

    #[test]
    fn global_zero_length_match() {
        // bsdtar test_option_s test 1_4: -s /f*/<~>/g
        // f* matches zero-length at every non-f position and consumes f's.
        let rules = SubstitutionRules::new(vec![SubstitutionRule::parse("/f*/<~>/g").unwrap()]);
        assert_eq!(
            rules.apply("in/d1/foo", false, false),
            "<>i<>n<>/<>d<>1<>/<f><>o<>o<>"
        );
    }

    #[test]
    fn global_dollar_anchor() {
        // $ matches at end of remaining; zero-length match at m.end() > 0
        assert_eq!(
            rules(&["/$/<END>/g"]).apply("ab", false, false),
            "ab<END><END>"
        );
    }

    #[test]
    fn multi_rule_symlink_s_flag() {
        // bsdtar test_option_s test 10: -s /realfile/foo/S -s /foo/realfile/
        // S flag: don't apply to symlink targets.
        // For regular files, rule 1 renames realfile→foo, rule 2 renames foo→realfile.
        // Position tracking prevents double-substitution.
        let rules = SubstitutionRules::new(vec![
            SubstitutionRule::parse("/realfile/foo/S").unwrap(),
            SubstitutionRule::parse("/foo/realfile/").unwrap(),
        ]);

        // Regular file: rule 1 matches, name advances; rule 2 sees remainder ""
        assert_eq!(rules.apply("in/d1/realfile", false, false), "in/d1/foo");
        // Regular file: rule 1 no match; rule 2 matches
        assert_eq!(rules.apply("in/d1/foo", false, false), "in/d1/realfile");
        // Symlink target: rule 1 skipped (S); rule 2 no match on "realfile"
        assert_eq!(rules.apply("realfile", true, false), "realfile");
    }

    #[test]
    fn escaped_tilde_produces_literal_tilde() {
        assert_eq!(
            rules(&["/foo/\\~/g"]).apply("foo baz foo", false, false),
            "~ baz ~"
        );
    }

    #[test]
    fn escaped_backslash() {
        assert_eq!(
            rules(&["/foo/\\\\/g"]).apply("foo baz foo", false, false),
            "\\ baz \\"
        );
    }

    #[test]
    fn escaped_backslash_before_digit() {
        // \\1 → literal \ then literal 1
        assert_eq!(
            rules(&["/(foo)/\\\\1/"]).apply("foo baz", false, false),
            "\\1 baz"
        );
    }

    #[test]
    fn escaped_backslash_before_tilde() {
        // \\~ → literal \ then full match
        assert_eq!(
            rules(&["/foo/\\\\~/g"]).apply("foo baz foo", false, false),
            "\\foo baz \\foo"
        );
    }

    #[test]
    fn backslash_zero_is_literal() {
        // \0 is NOT a backreference in bsdtar — output literally
        assert_eq!(
            rules(&["/foo/\\0/"]).apply("foo baz", false, false),
            "\\0 baz"
        );
    }

    #[test]
    fn global_substitution_consumes_position_for_next_rule() {
        // bsdtar test 14: -s /o/z/g -s /bar/baz/
        let rules = rules(&["/o/z/g", "/bar/baz/"]);
        assert_eq!(rules.apply("in/d1/foo", false, false), "in/d1/fzz");
        assert_eq!(rules.apply("in/d1/bar", false, false), "in/d1/baz");
    }

    #[test]
    fn singular_substitution_partial_position_for_next_rule() {
        // bsdtar test 14 singular variant: -s /o/z/ -s /bar/baz/
        let rules = rules(&["/o/z/", "/bar/baz/"]);
        assert_eq!(rules.apply("in/d1/foo", false, false), "in/d1/fzo");
        assert_eq!(rules.apply("in/d1/bar", false, false), "in/d1/baz");
    }

    #[test]
    fn selective_symlink_repointing() {
        // bsdtar test 11: -s /realfile/foo/sR — apply to symlinks only, skip regular
        let rules = rules(&["/realfile/foo/sR"]);
        assert_eq!(rules.apply("realfile", true, false), "foo");
        assert_eq!(rules.apply("realfile", false, false), "realfile");
    }

    #[test]
    fn hardlink_only_substitution() {
        let rules = rules(&["/target/newtarget/hR"]);
        assert_eq!(rules.apply("target", false, true), "newtarget");
        assert_eq!(rules.apply("target", false, false), "target");
    }

    #[test]
    fn no_match_returns_original() {
        assert_eq!(
            rules(&["/xyz/abc/"]).apply("in/d1/foo", false, false),
            "in/d1/foo"
        );
    }

    #[test]
    fn substitution_to_empty_string() {
        // bsdtar test 3: empty result
        assert_eq!(
            rules(&[",in/d1/foo,,"]).apply("in/d1/foo", false, false),
            ""
        );
    }

    #[test]
    fn skipped_rule_does_not_affect_position() {
        let rules = SubstitutionRules::new(vec![
            SubstitutionRule::parse("/foo/bar/").unwrap(),
            SubstitutionRule::parse("/bar/WRONG/R").unwrap(),
            SubstitutionRule::parse("/bar/baz/").unwrap(),
        ]);
        // Rule 2 skipped (R=skip regular); rule 3 sees remainder "" not "bar"
        assert_eq!(rules.apply("in/d1/foo", false, false), "in/d1/bar");
    }
}
