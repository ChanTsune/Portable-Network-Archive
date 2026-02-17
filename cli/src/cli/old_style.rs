use std::ffi::OsString;

/// `-J` is excluded from `SHORT_OPTIONS_WITH_ARG` because in bsdtar old-style syntax
/// it never takes an argument. (In new-style mode, clap handles the optional
/// compression level via `Option<Option<XzLevel>>`.)
/// Kept in sync with `StdioCommand` clap definition via unit test.
const SHORT_OPTIONS_WITH_ARG: &[char] = &['b', 'C', 'f', 'I', 's', 'T', 'X'];

/// Global long options that take a space-separated value.
/// `--flag=value` form is already a single token and needs no special handling.
/// Kept in sync with `Cli` clap definition via unit test.
const GLOBAL_LONG_OPTIONS_WITH_ARG: &[&str] = &["--color", "--log-level"];

/// Global long options that are boolean flags (no value).
/// Used to skip known global flags when detecting old-style candidates after `stdio`.
/// Kept in sync with `Cli` clap definition via unit test.
const GLOBAL_LONG_OPTIONS_WITHOUT_ARG: &[&str] = &["--quiet", "--unstable", "--verbose"];

/// Expands bsdtar old-style (dashless) arguments for the `experimental stdio` subcommand.
///
/// In old-style syntax the first word after `stdio` is a bundle of single-character
/// options without a leading dash. Options that take values consume subsequent
/// positional words. For example:
///
/// ```text
/// pna experimental stdio cvf archive.pna dir/
/// → pna experimental stdio -c -v -f archive.pna dir/
/// ```
///
/// Returns `args` unchanged when old-style is not detected.
pub fn expand_stdio_old_style_args(args: Vec<OsString>) -> Vec<OsString> {
    let Some(i) = skip_to_keyword(&args, 1, "experimental") else {
        return args;
    };
    let Some(after_stdio) = skip_to_keyword(&args, i, "stdio") else {
        return args;
    };
    // Skip only known global flags between `stdio` and the old-style candidate
    // (e.g. `--unstable`).  Any other flag indicates new-style syntax.
    let i = skip_known_global_flags(&args, after_stdio);

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

/// Skips only known global flags after `stdio`.  Any unrecognized flag
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
    fn no_stdio_passthrough() {
        let args = s(&["pna", "create", "-f", "archive.pna", "dir"]);
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn no_experimental_passthrough() {
        let args = s(&["pna", "stdio", "cvf", "archive.pna"]);
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn single_char_action() {
        let args = s(&["pna", "experimental", "stdio", "c"]);
        assert_eq!(
            expand_stdio_old_style_args(args),
            s(&["pna", "experimental", "stdio", "-c"]),
        );
    }

    #[test]
    fn flags_only() {
        let args = s(&["pna", "experimental", "stdio", "cv"]);
        assert_eq!(
            expand_stdio_old_style_args(args),
            s(&["pna", "experimental", "stdio", "-c", "-v"]),
        );
    }

    #[test]
    fn flags_with_arg() {
        let args = s(&["pna", "experimental", "stdio", "cvf", "archive.pna", "dir"]);
        assert_eq!(
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn unknown_flag_expanded_for_clap_to_reject() {
        // Unknown flags like `Q` are still expanded; clap rejects `-Q` with a clear error.
        let args = s(&["pna", "experimental", "stdio", "cQf", "archive.pna"]);
        assert_eq!(
            expand_stdio_old_style_args(args),
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
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn empty_candidate_passthrough() {
        let args = s(&["pna", "experimental", "stdio", ""]);
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn dash_prefix_passthrough() {
        let args = s(&["pna", "experimental", "stdio", "-cvf", "archive.pna", "dir"]);
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn at_prefix_passthrough() {
        let args = s(&["pna", "experimental", "stdio", "@archive.pna"]);
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn append_old_style() {
        let args = s(&["pna", "experimental", "stdio", "rf", "archive.pna", "dir"]);
        assert_eq!(
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
            s(&["pna", "experimental", "stdio", "-c", "-f"]),
        );
    }

    #[test]
    fn multiple_arg_options_insufficient_trailing() {
        let args = s(&["pna", "experimental", "stdio", "cfC", "archive.pna"]);
        assert_eq!(
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn binary_name_only() {
        let args = s(&["pna"]);
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn no_candidate_after_stdio() {
        let args = s(&["pna", "experimental", "stdio"]);
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn non_stdio_experimental_subcommand_passthrough() {
        let args = s(&["pna", "experimental", "delete", "cvf"]);
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn all_flags_no_positional() {
        let args = s(&["pna", "--quiet", "--verbose"]);
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn all_flags_after_experimental() {
        let args = s(&["pna", "experimental", "--flag1", "--flag2"]);
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_candidate_passthrough() {
        use std::os::unix::ffi::OsStringExt;
        let mut args = s(&["pna", "experimental", "stdio"]);
        args.push(OsString::from_vec(vec![0xFF, 0xFE]));
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn short_flag_after_stdio_means_new_style() {
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
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn unknown_long_flag_after_stdio_means_new_style() {
        // `--extract` is a stdio option, not a global flag — must be new-style.
        let args = s(&[
            "pna",
            "experimental",
            "stdio",
            "--unstable",
            "--extract",
            "--file",
            "archive.pna",
        ]);
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn double_dash_before_experimental() {
        let args = s(&["pna", "--", "experimental", "stdio", "cvf", "archive.pna"]);
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    #[test]
    fn double_dash_between_experimental_and_stdio() {
        let args = s(&["pna", "experimental", "--", "stdio", "cvf", "archive.pna"]);
        assert_eq!(expand_stdio_old_style_args(args.clone()), args);
    }

    fn takes_required_value(arg: &clap::Arg) -> bool {
        arg.get_action().takes_values()
            && arg
                .get_num_args()
                .is_none_or(|r: clap::builder::ValueRange| r.min_values() > 0)
    }

    #[test]
    fn short_options_with_arg_in_sync_with_clap() {
        let cmd = <crate::command::stdio::StdioCommand as clap::Args>::augment_args(
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
        let mut expected = SHORT_OPTIONS_WITH_ARG.to_vec();
        expected.sort();
        assert_eq!(
            clap_arg_shorts, expected,
            "SHORT_OPTIONS_WITH_ARG out of sync with StdioCommand"
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
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
            expand_stdio_old_style_args(args),
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
}
