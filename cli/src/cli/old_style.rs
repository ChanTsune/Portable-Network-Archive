use std::ffi::OsString;

/// Sentinel prefix for encoded `-C`/`--cd`/`--directory` arguments.
///
/// NUL bytes cannot appear in file paths, so this prefix unambiguously marks
/// a positional directory-change argument produced by [`encode_bsdtar_cd_args`].
pub(crate) const CD_SENTINEL: &str = "\0CD\0";

/// `-J` is excluded from `SHORT_OPTIONS_WITH_ARG` because in bsdtar old-style syntax
/// it never takes an argument. (In new-style mode, clap handles the optional
/// compression level via `Option<Option<XzLevel>>`.)
/// `-C` and `-W` are listed here for old-style expansion but are consumed by
/// `encode_bsdtar_cd_args` / `expand_bsdtar_w_option` before clap sees them.
/// Kept in sync with `BsdtarCommand` clap definition via unit test.
const SHORT_OPTIONS_WITH_ARG: &[char] = &['b', 'C', 'f', 'I', 's', 'T', 'W', 'X'];

/// Global long options that take a space-separated value.
/// `--flag=value` form is already a single token and needs no special handling.
/// Kept in sync with `Cli` clap definition via unit test.
const GLOBAL_LONG_OPTIONS_WITH_ARG: &[&str] = &["--color", "--log-level"];

/// Global long options that are boolean flags (no value).
/// Used to skip known global flags when detecting old-style candidates after the bsdtar subcommand.
/// Kept in sync with `Cli` clap definition via unit test.
const GLOBAL_LONG_OPTIONS_WITHOUT_ARG: &[&str] = &["--quiet", "--unstable", "--verbose"];

/// Finds the position after either `compat bsdtar` or `experimental stdio` keyword sequences.
fn find_bsdtar_subcommand(args: &[OsString]) -> Option<usize> {
    // Try "compat" -> "bsdtar" (primary path)
    if let Some(i) = skip_to_keyword(args, 1, "compat")
        && let Some(after) = skip_to_keyword(args, i, "bsdtar")
    {
        return Some(after);
    }
    // Fall back to "experimental" -> "stdio" (deprecated path)
    if let Some(i) = skip_to_keyword(args, 1, "experimental")
        && let Some(after) = skip_to_keyword(args, i, "stdio")
    {
        return Some(after);
    }
    None
}

/// Expands bsdtar old-style (dashless) arguments for the `compat bsdtar` subcommand
/// (and the deprecated `experimental stdio` subcommand).
///
/// In old-style syntax the first word after the subcommand is a bundle of single-character
/// options without a leading dash. Options that take values consume subsequent
/// positional words. For example:
///
/// ```text
/// pna compat bsdtar cvf archive.pna dir/
/// → pna compat bsdtar -c -v -f archive.pna dir/
/// ```
///
/// Returns `args` unchanged when old-style is not detected.
pub fn expand_bsdtar_old_style_args(args: Vec<OsString>) -> Vec<OsString> {
    let Some(after_subcommand) = find_bsdtar_subcommand(&args) else {
        return args;
    };
    // Skip only known global flags between the subcommand and the old-style candidate
    // (e.g. `--unstable`).  Any other flag indicates new-style syntax.
    let i = skip_known_global_flags(&args, after_subcommand);

    let candidate_str = args.get(i).and_then(|s| s.to_str()).unwrap_or_default();

    if !candidate_str.chars().all(|c| c.is_ascii_alphabetic()) {
        return args;
    }

    const ACTION_FLAGS: &[char] = &['c', 'x', 't', 'r', 'u'];
    if !candidate_str.chars().any(|c| ACTION_FLAGS.contains(&c)) {
        return args;
    }

    let mut result = args[..i].to_vec();
    let mut remaining = args[i + 1..].iter();

    for ch in candidate_str.chars() {
        result.push(OsString::from(format!("-{ch}")));
        if SHORT_OPTIONS_WITH_ARG.contains(&ch)
            && let Some(value) = remaining.next()
        {
            result.push(value.clone());
        }
    }

    result.extend(remaining.cloned());
    result
}

/// Expands bsdtar `-W <long_option>` syntax for the `compat bsdtar` subcommand
/// (and the deprecated `experimental stdio` subcommand).
///
/// In bsdtar, `-W <name>` is equivalent to `--<name>`, and `-W <name>=<value>` is
/// equivalent to `--<name>=<value>`.
pub fn expand_bsdtar_w_option(args: Vec<OsString>) -> Vec<OsString> {
    let Some(after_subcommand) = find_bsdtar_subcommand(&args) else {
        return args;
    };

    let mut result = args[..after_subcommand].to_vec();
    let mut iter = args[after_subcommand..].iter();
    let mut past_double_dash = false;
    while let Some(arg) = iter.next() {
        if !past_double_dash && arg == "--" {
            past_double_dash = true;
            result.push(arg.clone());
        } else if !past_double_dash && arg == "-W" {
            if let Some(next) = iter.next() {
                let mut long_form = OsString::from("--");
                long_form.push(next);
                result.push(long_form);
            } else {
                result.push(arg.clone());
            }
        } else {
            result.push(arg.clone());
        }
    }
    result
}

/// Encodes `-C`/`--cd`/`--directory` arguments into sentinel-prefixed positional tokens
/// for the `compat bsdtar` subcommand (and the deprecated `experimental stdio` subcommand).
///
/// This converts directory-change options into `\0CD\0{dir}` tokens so that
/// later processing stages can treat them as positional arguments that carry
/// their directory value inline.
///
/// Recognized forms (after the bsdtar subcommand position):
/// - `-C dir` (separate tokens)
/// - `-Cdir` (concatenated short form)
/// - `--cd dir` / `--directory dir` (long form with separate value)
/// - `--cd=dir` / `--directory=dir` (long form with `=`)
///
/// Arguments after `--` are never encoded.
///
/// Returns `args` unchanged when the bsdtar subcommand is not detected.
pub fn encode_bsdtar_cd_args(args: Vec<OsString>) -> Vec<OsString> {
    fn make_cd_sentinel(value: &std::ffi::OsStr) -> OsString {
        let mut s = OsString::with_capacity(CD_SENTINEL.len() + value.len());
        s.push(CD_SENTINEL);
        s.push(value);
        s
    }

    let Some(after_subcommand) = find_bsdtar_subcommand(&args) else {
        return args;
    };

    let mut result = args[..after_subcommand].to_vec();
    let mut iter = args[after_subcommand..].iter();
    let mut past_double_dash = false;

    while let Some(arg) = iter.next() {
        if past_double_dash {
            result.push(arg.clone());
            continue;
        }

        if arg == "--" {
            past_double_dash = true;
            result.push(arg.clone());
            continue;
        }

        let bytes = arg.as_encoded_bytes();

        // `--cd=value` or `--directory=value`
        if let Some(rest) = bytes.strip_prefix(b"--cd=") {
            // SAFETY: rest comes from valid OsString bytes after stripping an ASCII prefix
            result.push(make_cd_sentinel(unsafe {
                std::ffi::OsStr::from_encoded_bytes_unchecked(rest)
            }));
            continue;
        }
        if let Some(rest) = bytes.strip_prefix(b"--directory=") {
            result.push(make_cd_sentinel(unsafe {
                std::ffi::OsStr::from_encoded_bytes_unchecked(rest)
            }));
            continue;
        }

        // `--cd dir` or `--directory dir`
        if arg == "--cd" || arg == "--directory" {
            if let Some(value) = iter.next() {
                result.push(make_cd_sentinel(value));
            } else {
                // Trailing `--cd` with no value — pass through for clap to report an error
                result.push(arg.clone());
            }
            continue;
        }

        // `-C dir` (exactly `-C`)
        if arg == "-C" {
            if let Some(value) = iter.next() {
                result.push(make_cd_sentinel(value));
            } else {
                result.push(arg.clone());
            }
            continue;
        }

        // `-Cdir` (concatenated short form)
        if let Some(rest) = bytes.strip_prefix(b"-C")
            && !rest.is_empty()
            && !rest.starts_with(b"-")
        {
            result.push(make_cd_sentinel(unsafe {
                std::ffi::OsStr::from_encoded_bytes_unchecked(rest)
            }));
            continue;
        }

        result.push(arg.clone());
    }

    result
}

/// Skips leading flags (consuming values for known global options like
/// `--color always`) then checks for the expected keyword.
/// Returns the index after the keyword, or `None` if not found.
fn skip_to_keyword(args: &[OsString], i: usize, keyword: &str) -> Option<usize> {
    let i = skip_leading_flags(args, i);
    (i < args.len() && args[i] == keyword).then_some(i + 1)
}

fn skip_leading_flags(args: &[OsString], mut i: usize) -> usize {
    while i < args.len() && args[i].as_encoded_bytes().first() == Some(&b'-') && args[i] != "--" {
        let has_value = GLOBAL_LONG_OPTIONS_WITH_ARG
            .iter()
            .any(|&opt| args[i] == opt);
        i += 1;
        if has_value && i < args.len() {
            i += 1;
        }
    }
    i
}

/// Skips only known global flags after the bsdtar subcommand.  Any unrecognized flag
/// (short like `-x` or long like `--extract`) means new-style syntax.
fn skip_known_global_flags(args: &[OsString], mut i: usize) -> usize {
    while i < args.len() {
        if GLOBAL_LONG_OPTIONS_WITH_ARG
            .iter()
            .any(|&opt| args[i] == opt)
        {
            // `--color always` — skip the flag and its value.
            i += 1;
            if i < args.len() {
                i += 1;
            }
        } else if GLOBAL_LONG_OPTIONS_WITHOUT_ARG
            .iter()
            .any(|&opt| args[i] == opt)
        {
            // `--unstable`, `--quiet`, `--verbose` — skip the flag.
            i += 1;
        } else if args[i].as_encoded_bytes().starts_with(b"--")
            && args[i] != "--"
            && GLOBAL_LONG_OPTIONS_WITH_ARG.iter().any(|&opt| {
                args[i].as_encoded_bytes().starts_with(opt.as_bytes())
                    && args[i].as_encoded_bytes().get(opt.len()) == Some(&b'=')
            })
        {
            // `--color=always` — single token, skip it.
            i += 1;
        } else {
            break;
        }
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(args: &[&str]) -> Vec<OsString> {
        args.iter().map(OsString::from).collect()
    }

    #[test]
    fn unrelated_command_passthrough() {
        let args = s(&["pna", "create", "-f", "archive.pna", "dir"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn bare_stdio_without_parent_passthrough() {
        let args = s(&["pna", "stdio", "cvf", "archive.pna"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn single_char_action() {
        let args = s(&["pna", "experimental", "stdio", "c"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&["pna", "experimental", "stdio", "-c"]),
        );
    }

    #[test]
    fn flags_only() {
        let args = s(&["pna", "experimental", "stdio", "cv"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&["pna", "experimental", "stdio", "-c", "-v"]),
        );
    }

    #[test]
    fn flags_with_arg() {
        let args = s(&["pna", "experimental", "stdio", "cvf", "archive.pna", "dir"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-c",
                "-v",
                "-f",
                "archive.pna",
                "dir",
            ]),
        );
    }

    #[test]
    fn multiple_arg_options() {
        let args = s(&[
            "pna",
            "experimental",
            "stdio",
            "cfC",
            "archive.pna",
            "/tmp",
            "dir",
        ]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-c",
                "-f",
                "archive.pna",
                "-C",
                "/tmp",
                "dir",
            ]),
        );
    }

    #[test]
    fn j_treated_as_flag() {
        let args = s(&["pna", "experimental", "stdio", "cJvf", "archive.pna", "dir"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-c",
                "-J",
                "-v",
                "-f",
                "archive.pna",
                "dir",
            ]),
        );
    }

    #[test]
    fn no_action_flag_passthrough() {
        let args = s(&["pna", "experimental", "stdio", "vf", "archive.pna"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn unknown_flag_expanded_for_clap_to_reject() {
        // Unknown flags like `Q` are still expanded; clap rejects `-Q` with a clear error.
        let args = s(&["pna", "experimental", "stdio", "cQf", "archive.pna"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-c",
                "-Q",
                "-f",
                "archive.pna",
            ]),
        );
    }

    #[test]
    fn non_alpha_char_passthrough() {
        let args = s(&["pna", "experimental", "stdio", "c2f", "archive.pna"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn empty_candidate_passthrough() {
        let args = s(&["pna", "experimental", "stdio", ""]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn dash_prefix_passthrough() {
        let args = s(&["pna", "experimental", "stdio", "-cvf", "archive.pna", "dir"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn at_prefix_passthrough() {
        let args = s(&["pna", "experimental", "stdio", "@archive.pna"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn append_old_style() {
        let args = s(&["pna", "experimental", "stdio", "rf", "archive.pna", "dir"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-r",
                "-f",
                "archive.pna",
                "dir",
            ]),
        );
    }

    #[test]
    fn update_old_style() {
        let args = s(&["pna", "experimental", "stdio", "uf", "archive.pna", "dir"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-u",
                "-f",
                "archive.pna",
                "dir",
            ]),
        );
    }

    #[test]
    fn missing_arg_word() {
        let args = s(&["pna", "experimental", "stdio", "cf"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&["pna", "experimental", "stdio", "-c", "-f"]),
        );
    }

    #[test]
    fn multiple_arg_options_insufficient_trailing() {
        let args = s(&["pna", "experimental", "stdio", "cfC", "archive.pna"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-c",
                "-f",
                "archive.pna",
                "-C",
            ]),
        );
    }

    #[test]
    fn interleaved_global_flags() {
        let args = s(&[
            "pna",
            "--quiet",
            "experimental",
            "stdio",
            "cvf",
            "archive.pna",
            "dir",
        ]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "--quiet",
                "experimental",
                "stdio",
                "-c",
                "-v",
                "-f",
                "archive.pna",
                "dir",
            ]),
        );
    }

    #[test]
    fn global_flag_between_experimental_and_stdio() {
        let args = s(&[
            "pna",
            "experimental",
            "--unstable",
            "stdio",
            "tf",
            "archive.pna",
        ]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "--unstable",
                "stdio",
                "-t",
                "-f",
                "archive.pna",
            ]),
        );
    }

    #[test]
    fn old_style_followed_by_new_style() {
        let args = s(&[
            "pna",
            "experimental",
            "stdio",
            "cvf",
            "archive.pna",
            "--xz",
            "dir",
        ]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-c",
                "-v",
                "-f",
                "archive.pna",
                "--xz",
                "dir",
            ]),
        );
    }

    #[test]
    fn extract_old_style() {
        let args = s(&["pna", "experimental", "stdio", "xvf", "archive.pna"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-x",
                "-v",
                "-f",
                "archive.pna",
            ]),
        );
    }

    #[test]
    fn list_old_style() {
        let args = s(&["pna", "experimental", "stdio", "tf", "archive.pna"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&["pna", "experimental", "stdio", "-t", "-f", "archive.pna",]),
        );
    }

    #[test]
    fn unstable_between_stdio_and_candidate() {
        let args = s(&[
            "pna",
            "experimental",
            "stdio",
            "--unstable",
            "cvf",
            "archive.pna",
            "dir",
        ]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "--unstable",
                "-c",
                "-v",
                "-f",
                "archive.pna",
                "dir",
            ]),
        );
    }

    #[test]
    fn tvf_verbose_list() {
        let args = s(&["pna", "experimental", "stdio", "tvf", "archive.pna"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-t",
                "-v",
                "-f",
                "archive.pna"
            ]),
        );
    }

    #[test]
    fn extract_with_directory_change_in_bundle() {
        let args = s(&[
            "pna",
            "experimental",
            "stdio",
            "xvfC",
            "archive.pna",
            "/tmp",
        ]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-x",
                "-v",
                "-f",
                "archive.pna",
                "-C",
                "/tmp",
            ]),
        );
    }

    #[test]
    fn empty_args() {
        let args: Vec<OsString> = vec![];
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn binary_name_only() {
        let args = s(&["pna"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn no_candidate_after_stdio() {
        let args = s(&["pna", "experimental", "stdio"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn non_stdio_experimental_subcommand_passthrough() {
        let args = s(&["pna", "experimental", "delete", "cvf"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn all_flags_no_positional() {
        let args = s(&["pna", "--quiet", "--verbose"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn all_flags_after_experimental() {
        let args = s(&["pna", "experimental", "--flag1", "--flag2"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_candidate_passthrough() {
        use std::os::unix::ffi::OsStringExt;
        let mut args = s(&["pna", "experimental", "stdio"]);
        args.push(OsString::from_vec(vec![0xFF, 0xFE]));
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn short_flag_after_subcommand_means_new_style() {
        // `-x -C target ...` is new-style; must NOT expand `target` as old-style.
        let args = s(&[
            "pna",
            "experimental",
            "stdio",
            "--unstable",
            "-x",
            "-C",
            "target",
            "--strip-components",
            "2",
            "-f",
            "test.tar",
        ]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn unknown_long_flag_after_subcommand_means_new_style() {
        // `--extract` is a bsdtar option, not a global flag — must be new-style.
        let args = s(&[
            "pna",
            "experimental",
            "stdio",
            "--unstable",
            "--extract",
            "--file",
            "archive.pna",
        ]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn double_dash_before_experimental() {
        let args = s(&["pna", "--", "experimental", "stdio", "cvf", "archive.pna"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn double_dash_between_experimental_and_stdio() {
        let args = s(&["pna", "experimental", "--", "stdio", "cvf", "archive.pna"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    fn takes_required_value(arg: &clap::Arg) -> bool {
        arg.get_action().takes_values()
            && arg
                .get_num_args()
                .is_none_or(|r: clap::builder::ValueRange| r.min_values() > 0)
    }

    #[test]
    fn short_options_with_arg_in_sync_with_clap() {
        let cmd = <crate::command::bsdtar::BsdtarCommand as clap::Args>::augment_args(
            clap::Command::new("test"),
        );
        let mut clap_arg_shorts: Vec<char> = cmd
            .get_arguments()
            .filter(|arg| takes_required_value(arg))
            .flat_map(|arg| {
                arg.get_short()
                    .into_iter()
                    .chain(arg.get_all_short_aliases().unwrap_or_default())
            })
            .collect();
        clap_arg_shorts.sort();
        clap_arg_shorts.dedup();
        // `C` and `W` are handled by preprocessing (encode_bsdtar_cd_args /
        // expand_bsdtar_w_option), not by clap
        let mut expected: Vec<char> = SHORT_OPTIONS_WITH_ARG
            .iter()
            .copied()
            .filter(|&c| c != 'C' && c != 'W')
            .collect();
        expected.sort();
        assert_eq!(
            clap_arg_shorts, expected,
            "SHORT_OPTIONS_WITH_ARG out of sync with BsdtarCommand"
        );
    }

    #[test]
    fn global_long_options_with_arg_in_sync_with_clap() {
        let cmd = <crate::cli::Cli as clap::CommandFactory>::command();
        let mut clap_global_longs: Vec<String> = cmd
            .get_arguments()
            .filter(|arg| arg.is_global_set() && takes_required_value(arg))
            .flat_map(|arg| {
                arg.get_long()
                    .into_iter()
                    .chain(arg.get_all_aliases().unwrap_or_default())
            })
            .map(|s| format!("--{s}"))
            .collect();
        clap_global_longs.sort();
        clap_global_longs.dedup();
        let mut expected = GLOBAL_LONG_OPTIONS_WITH_ARG.to_vec();
        expected.sort();
        assert_eq!(
            clap_global_longs, expected,
            "GLOBAL_LONG_OPTIONS_WITH_ARG out of sync with Cli global args"
        );
    }

    #[test]
    fn global_long_options_without_arg_in_sync_with_clap() {
        let cmd = <crate::cli::Cli as clap::CommandFactory>::command();
        let mut clap_global_flags: Vec<String> = cmd
            .get_arguments()
            .filter(|arg| arg.is_global_set() && !takes_required_value(arg))
            .flat_map(|arg| {
                arg.get_long()
                    .into_iter()
                    .chain(arg.get_all_aliases().unwrap_or_default())
            })
            .map(|s| format!("--{s}"))
            .collect();
        clap_global_flags.sort();
        clap_global_flags.dedup();
        let mut expected = GLOBAL_LONG_OPTIONS_WITHOUT_ARG.to_vec();
        expected.sort();
        assert_eq!(
            clap_global_flags, expected,
            "GLOBAL_LONG_OPTIONS_WITHOUT_ARG out of sync with Cli global args"
        );
    }

    #[test]
    fn global_flag_with_space_value_before_experimental() {
        let args = s(&[
            "pna",
            "--color",
            "always",
            "experimental",
            "stdio",
            "cvf",
            "archive.pna",
            "dir",
        ]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "--color",
                "always",
                "experimental",
                "stdio",
                "-c",
                "-v",
                "-f",
                "archive.pna",
                "dir",
            ]),
        );
    }

    #[test]
    fn global_flag_with_eq_value_before_experimental() {
        let args = s(&[
            "pna",
            "--color=always",
            "experimental",
            "stdio",
            "cvf",
            "archive.pna",
            "dir",
        ]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "--color=always",
                "experimental",
                "stdio",
                "-c",
                "-v",
                "-f",
                "archive.pna",
                "dir",
            ]),
        );
    }

    #[test]
    fn global_flag_with_space_value_between_experimental_and_stdio() {
        let args = s(&[
            "pna",
            "experimental",
            "--color",
            "always",
            "stdio",
            "tf",
            "archive.pna",
        ]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "experimental",
                "--color",
                "always",
                "stdio",
                "-t",
                "-f",
                "archive.pna",
            ]),
        );
    }

    #[test]
    fn multiple_global_flags_with_value() {
        let args = s(&[
            "pna",
            "--quiet",
            "--color",
            "never",
            "experimental",
            "stdio",
            "xf",
            "archive.pna",
        ]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "--quiet",
                "--color",
                "never",
                "experimental",
                "stdio",
                "-x",
                "-f",
                "archive.pna",
            ]),
        );
    }

    #[test]
    fn w_option_help() {
        let args = s(&["pna", "experimental", "stdio", "-W", "help"]);
        assert_eq!(
            expand_bsdtar_w_option(args),
            s(&["pna", "experimental", "stdio", "--help"]),
        );
    }

    #[test]
    fn w_option_version() {
        let args = s(&["pna", "experimental", "stdio", "-W", "version"]);
        assert_eq!(
            expand_bsdtar_w_option(args),
            s(&["pna", "experimental", "stdio", "--version"]),
        );
    }

    #[test]
    fn w_option_with_equals_value() {
        let args = s(&[
            "pna",
            "experimental",
            "stdio",
            "-x",
            "-W",
            "strip-components=3",
            "-f",
            "a.pna",
        ]);
        assert_eq!(
            expand_bsdtar_w_option(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-x",
                "--strip-components=3",
                "-f",
                "a.pna",
            ]),
        );
    }

    #[test]
    fn w_option_multiple() {
        let args = s(&[
            "pna",
            "experimental",
            "stdio",
            "-x",
            "-W",
            "same-owner",
            "-W",
            "keep-old-files",
            "-f",
            "a.pna",
        ]);
        assert_eq!(
            expand_bsdtar_w_option(args),
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-x",
                "--same-owner",
                "--keep-old-files",
                "-f",
                "a.pna",
            ]),
        );
    }

    #[test]
    fn w_option_trailing_no_arg() {
        let args = s(&["pna", "experimental", "stdio", "-x", "-W"]);
        assert_eq!(
            expand_bsdtar_w_option(args),
            s(&["pna", "experimental", "stdio", "-x", "-W"]),
        );
    }

    #[test]
    fn w_option_no_stdio_passthrough() {
        let args = s(&["pna", "create", "-W", "help"]);
        assert_eq!(expand_bsdtar_w_option(args.clone()), args);
    }

    #[test]
    fn w_option_with_unstable() {
        let args = s(&["pna", "experimental", "stdio", "--unstable", "-W", "help"]);
        assert_eq!(
            expand_bsdtar_w_option(args),
            s(&["pna", "experimental", "stdio", "--unstable", "--help"]),
        );
    }

    #[test]
    fn w_option_after_old_style_expansion() {
        let args = s(&["pna", "experimental", "stdio", "-x", "-W", "help"]);
        assert_eq!(
            expand_bsdtar_w_option(args),
            s(&["pna", "experimental", "stdio", "-x", "--help"]),
        );
    }

    #[test]
    fn w_option_after_double_dash_passthrough() {
        let args = s(&["pna", "experimental", "stdio", "-x", "--", "-W", "file"]);
        assert_eq!(expand_bsdtar_w_option(args.clone()), args);
    }

    #[test]
    fn w_option_double_dash_before_experimental_passthrough() {
        let args = s(&["pna", "--", "experimental", "stdio", "-W", "help"]);
        assert_eq!(expand_bsdtar_w_option(args.clone()), args);
    }

    #[test]
    fn w_option_double_dash_between_experimental_and_stdio() {
        let args = s(&["pna", "experimental", "--", "stdio", "-W", "help"]);
        assert_eq!(expand_bsdtar_w_option(args.clone()), args);
    }

    #[test]
    fn w_option_non_stdio_experimental_subcommand_passthrough() {
        let args = s(&["pna", "experimental", "delete", "-W", "help"]);
        assert_eq!(expand_bsdtar_w_option(args.clone()), args);
    }

    #[test]
    fn w_option_empty_args() {
        let args: Vec<OsString> = vec![];
        assert_eq!(expand_bsdtar_w_option(args.clone()), args);
    }

    #[test]
    fn w_option_binary_name_only() {
        let args = s(&["pna"]);
        assert_eq!(expand_bsdtar_w_option(args.clone()), args);
    }

    #[test]
    fn w_option_no_args_after_stdio() {
        let args = s(&["pna", "experimental", "stdio"]);
        assert_eq!(expand_bsdtar_w_option(args.clone()), args);
    }

    #[test]
    fn w_option_single_token_not_matched() {
        let args = s(&["pna", "experimental", "stdio", "-Whelp"]);
        assert_eq!(expand_bsdtar_w_option(args.clone()), args);
    }

    #[test]
    fn w_option_global_flag_with_value_before_experimental() {
        let args = s(&[
            "pna",
            "--color",
            "always",
            "experimental",
            "stdio",
            "-W",
            "help",
        ]);
        assert_eq!(
            expand_bsdtar_w_option(args),
            s(&[
                "pna",
                "--color",
                "always",
                "experimental",
                "stdio",
                "--help",
            ]),
        );
    }

    #[test]
    fn w_option_global_flag_between_experimental_and_stdio() {
        let args = s(&[
            "pna",
            "experimental",
            "--color",
            "always",
            "stdio",
            "-W",
            "help",
        ]);
        assert_eq!(
            expand_bsdtar_w_option(args),
            s(&[
                "pna",
                "experimental",
                "--color",
                "always",
                "stdio",
                "--help",
            ]),
        );
    }

    #[cfg(unix)]
    #[test]
    fn w_option_non_utf8_value() {
        use std::os::unix::ffi::OsStringExt;
        let mut args = s(&["pna", "experimental", "stdio", "-W"]);
        args.push(OsString::from_vec(vec![0xFF, 0xFE]));
        let result = expand_bsdtar_w_option(args);
        let mut expected_long = OsString::from("--");
        expected_long.push(OsString::from_vec(vec![0xFF, 0xFE]));
        let mut expected = s(&["pna", "experimental", "stdio"]);
        expected.push(expected_long);
        assert_eq!(result, expected);
    }

    #[test]
    fn pipeline_old_style_with_w_and_value() {
        let args = s(&[
            "pna",
            "experimental",
            "stdio",
            "xW",
            "same-owner",
            "-f",
            "a.pna",
        ]);
        let args = expand_bsdtar_old_style_args(args);
        let args = expand_bsdtar_w_option(args);
        assert_eq!(
            args,
            s(&[
                "pna",
                "experimental",
                "stdio",
                "-x",
                "--same-owner",
                "-f",
                "a.pna"
            ]),
        );
    }

    #[test]
    fn compat_bsdtar_old_style() {
        let args = s(&["pna", "compat", "bsdtar", "cvf", "archive.pna", "dir"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "compat",
                "bsdtar",
                "-c",
                "-v",
                "-f",
                "archive.pna",
                "dir",
            ]),
        );
    }

    #[test]
    fn compat_bsdtar_single_action() {
        let args = s(&["pna", "compat", "bsdtar", "t"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&["pna", "compat", "bsdtar", "-t"]),
        );
    }

    #[test]
    fn compat_bsdtar_with_global_flags() {
        let args = s(&["pna", "--quiet", "compat", "bsdtar", "xvf", "archive.pna"]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "--quiet",
                "compat",
                "bsdtar",
                "-x",
                "-v",
                "-f",
                "archive.pna",
            ]),
        );
    }

    #[test]
    fn compat_bsdtar_new_style_passthrough() {
        let args = s(&[
            "pna",
            "compat",
            "bsdtar",
            "-c",
            "-v",
            "-f",
            "archive.pna",
            "dir",
        ]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn compat_bsdtar_no_action_passthrough() {
        let args = s(&["pna", "compat", "bsdtar", "vf", "archive.pna"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn compat_bsdtar_w_option_help() {
        let args = s(&["pna", "compat", "bsdtar", "-W", "help"]);
        assert_eq!(
            expand_bsdtar_w_option(args),
            s(&["pna", "compat", "bsdtar", "--help"]),
        );
    }

    #[test]
    fn compat_bsdtar_w_option_with_value() {
        let args = s(&[
            "pna",
            "compat",
            "bsdtar",
            "-x",
            "-W",
            "strip-components=3",
            "-f",
            "a.pna",
        ]);
        assert_eq!(
            expand_bsdtar_w_option(args),
            s(&[
                "pna",
                "compat",
                "bsdtar",
                "-x",
                "--strip-components=3",
                "-f",
                "a.pna",
            ]),
        );
    }

    #[test]
    fn compat_bsdtar_w_option_no_compat_passthrough() {
        let args = s(&["pna", "create", "-W", "help"]);
        assert_eq!(expand_bsdtar_w_option(args.clone()), args);
    }

    #[test]
    fn compat_without_bsdtar_passthrough() {
        let args = s(&["pna", "compat", "cvf", "archive.pna"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn compat_wrong_sub_keyword_passthrough() {
        let args = s(&["pna", "compat", "stdio", "cvf", "archive.pna"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn bare_bsdtar_without_compat_passthrough() {
        let args = s(&["pna", "bsdtar", "cvf", "archive.pna"]);
        assert_eq!(expand_bsdtar_old_style_args(args.clone()), args);
    }

    #[test]
    fn compat_bsdtar_global_flag_between_compat_and_bsdtar() {
        let args = s(&[
            "pna",
            "compat",
            "--quiet",
            "bsdtar",
            "cvf",
            "archive.pna",
            "dir",
        ]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "compat",
                "--quiet",
                "bsdtar",
                "-c",
                "-v",
                "-f",
                "archive.pna",
                "dir",
            ]),
        );
    }

    #[test]
    fn compat_bsdtar_unstable_after_bsdtar() {
        let args = s(&[
            "pna",
            "compat",
            "bsdtar",
            "--unstable",
            "cvf",
            "archive.pna",
            "dir",
        ]);
        assert_eq!(
            expand_bsdtar_old_style_args(args),
            s(&[
                "pna",
                "compat",
                "bsdtar",
                "--unstable",
                "-c",
                "-v",
                "-f",
                "archive.pna",
                "dir",
            ]),
        );
    }

    #[test]
    fn compat_bsdtar_pipeline_old_style_with_w() {
        let args = s(&["pna", "compat", "bsdtar", "xW", "same-owner", "-f", "a.pna"]);
        let args = expand_bsdtar_old_style_args(args);
        let args = expand_bsdtar_w_option(args);
        assert_eq!(
            args,
            s(&[
                "pna",
                "compat",
                "bsdtar",
                "-x",
                "--same-owner",
                "-f",
                "a.pna"
            ]),
        );
    }

    #[test]
    fn compat_bsdtar_w_option_after_double_dash_passthrough() {
        let args = s(&["pna", "compat", "bsdtar", "-x", "--", "-W", "file"]);
        assert_eq!(expand_bsdtar_w_option(args.clone()), args);
    }

    #[test]
    fn encode_cd_separate_form() {
        let args = s(&[
            "pna", "compat", "bsdtar", "-c", "-f", "out", "-C", "dir", "file",
        ]);
        let result = encode_bsdtar_cd_args(args);
        assert_eq!(
            result,
            s(&[
                "pna",
                "compat",
                "bsdtar",
                "-c",
                "-f",
                "out",
                "\0CD\0dir",
                "file"
            ]),
        );
    }

    #[test]
    fn encode_cd_concatenated_form() {
        let args = s(&["pna", "compat", "bsdtar", "-c", "-Cdir", "file"]);
        let result = encode_bsdtar_cd_args(args);
        assert_eq!(
            result,
            s(&["pna", "compat", "bsdtar", "-c", "\0CD\0dir", "file"]),
        );
    }

    #[test]
    fn encode_cd_long_form() {
        let args = s(&["pna", "compat", "bsdtar", "--cd", "dir", "file"]);
        let result = encode_bsdtar_cd_args(args);
        assert_eq!(result, s(&["pna", "compat", "bsdtar", "\0CD\0dir", "file"]),);
    }

    #[test]
    fn encode_cd_long_eq_form() {
        let args = s(&["pna", "compat", "bsdtar", "--directory=dir", "file"]);
        let result = encode_bsdtar_cd_args(args);
        assert_eq!(result, s(&["pna", "compat", "bsdtar", "\0CD\0dir", "file"]),);
    }

    #[test]
    fn encode_cd_multiple() {
        let args = s(&[
            "pna", "compat", "bsdtar", "-C", "d1", "f1", "-C", "d2", "f2",
        ]);
        let result = encode_bsdtar_cd_args(args);
        assert_eq!(
            result,
            s(&[
                "pna", "compat", "bsdtar", "\0CD\0d1", "f1", "\0CD\0d2", "f2"
            ]),
        );
    }

    #[test]
    fn encode_cd_after_double_dash() {
        let args = s(&["pna", "compat", "bsdtar", "--", "-C", "dir"]);
        let result = encode_bsdtar_cd_args(args);
        assert_eq!(result, s(&["pna", "compat", "bsdtar", "--", "-C", "dir"]),);
    }

    #[test]
    fn encode_cd_not_bsdtar() {
        let args = s(&["pna", "create", "-C", "dir", "file"]);
        let result = encode_bsdtar_cd_args(args);
        assert_eq!(result, s(&["pna", "create", "-C", "dir", "file"]));
    }

    #[test]
    fn encode_cd_experimental_stdio() {
        let args = s(&["pna", "experimental", "stdio", "-C", "dir", "file"]);
        let result = encode_bsdtar_cd_args(args);
        assert_eq!(
            result,
            s(&["pna", "experimental", "stdio", "\0CD\0dir", "file"]),
        );
    }

    #[test]
    fn encode_cd_with_unstable_flag() {
        let args = s(&["pna", "compat", "bsdtar", "--unstable", "-C", "dir", "file"]);
        let result = encode_bsdtar_cd_args(args);
        assert_eq!(
            result,
            s(&["pna", "compat", "bsdtar", "--unstable", "\0CD\0dir", "file"]),
        );
    }

    #[test]
    fn encode_cd_after_old_style_c_cf_expansion() {
        // Simulates output of expand_bsdtar_old_style_args for "cCf dir archive file"
        let args = s(&[
            "pna", "compat", "bsdtar", "-c", "-C", "dir", "-f", "archive", "file",
        ]);
        let result = encode_bsdtar_cd_args(args);
        assert_eq!(
            result,
            s(&[
                "pna",
                "compat",
                "bsdtar",
                "-c",
                "\0CD\0dir",
                "-f",
                "archive",
                "file",
            ]),
        );
    }
}
