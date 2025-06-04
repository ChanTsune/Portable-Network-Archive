use crate::{
    cli::{PasswordArgs, SolidEntriesTransformStrategy, SolidEntriesTransformStrategyArgs},
    command::{
        ask_password,
        commons::{
            collect_split_archives, run_transform_entry, TransformStrategyKeepSolid,
            TransformStrategyUnSolid,
        },
        Command,
    },
    utils::{GlobPatterns, PathPartExt},
};
use bitflags::bitflags;
use clap::{Parser, ValueHint};
use pna::NormalEntry;
use std::{io, path::PathBuf, str::FromStr};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ChmodCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(help = "mode")]
    mode: Mode,
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[command(flatten)]
    transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for ChmodCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        archive_chmod(self)
    }
}

fn archive_chmod(args: ChmodCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
    if args.files.is_empty() {
        return Ok(());
    }
    let globs = GlobPatterns::new(args.files)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let archives = collect_split_archives(&args.archive)?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => run_transform_entry(
            args.archive.remove_part().unwrap(),
            archives,
            || password.as_deref(),
            |entry| {
                let entry = entry?;
                if globs.matches_any(entry.header().path()) {
                    Ok(Some(transform_entry(entry, &args.mode)))
                } else {
                    Ok(Some(entry))
                }
            },
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => run_transform_entry(
            args.archive.remove_part().unwrap(),
            archives,
            || password.as_deref(),
            |entry| {
                let entry = entry?;
                if globs.matches_any(entry.header().path()) {
                    Ok(Some(transform_entry(entry, &args.mode)))
                } else {
                    Ok(Some(entry))
                }
            },
            TransformStrategyKeepSolid,
        ),
    }
}

#[inline]
fn transform_entry<T>(entry: NormalEntry<T>, mode: &Mode) -> NormalEntry<T> {
    let metadata = entry.metadata().clone();
    let permission = metadata.permission().map(|p| {
        let mode = mode.apply_to(p.permissions());
        pna::Permission::new(p.uid(), p.uname().into(), p.gid(), p.gname().into(), mode)
    });
    entry.with_metadata(metadata.with_permission(permission))
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub(crate) struct Who: u8 {
        const User = 0b001;
        const Group = 0b010;
        const Other = 0b100;
        const All = 0b111;
    }
}

impl Who {
    #[inline]
    const fn apply_to(&self, n: u16) -> u16 {
        let mut result = 0;
        if self.contains(Who::User) {
            result |= n << 6;
        }
        if self.contains(Who::Group) {
            result |= n << 3;
        }
        if self.contains(Who::Other) {
            result |= n;
        }
        result
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Mode {
    Numeric(u16),
    Equal(Who, u8),
    Plus(Who, u8),
    Minus(Who, u8),
}

impl Mode {
    const OWNER_MASK: u16 = 0o700;
    const GROUP_MASK: u16 = 0o070;
    const OTHER_MASK: u16 = 0o007;
    #[inline]
    pub(crate) const fn apply_to(&self, mode: u16) -> u16 {
        match self {
            Mode::Numeric(mode) => *mode,
            Mode::Equal(t, m) => {
                let owner_mode = if t.contains(Who::User) {
                    Who::User.apply_to(*m as u16)
                } else {
                    mode & Self::OWNER_MASK
                };
                let group_mode = if t.contains(Who::Group) {
                    Who::Group.apply_to(*m as u16)
                } else {
                    mode & Self::GROUP_MASK
                };
                let other_mode = if t.contains(Who::Other) {
                    Who::Other.apply_to(*m as u16)
                } else {
                    mode & Self::OTHER_MASK
                };
                owner_mode | group_mode | other_mode
            }
            Mode::Plus(t, m) => mode | t.apply_to(*m as u16),
            Mode::Minus(t, m) => mode & !t.apply_to(*m as u16),
        }
    }
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[inline]
        fn parse_mode(chars: impl Iterator<Item = char>) -> Result<u8, <Mode as FromStr>::Err> {
            let mut mode = 0;
            for c in chars {
                match c {
                    'x' => mode |= 1,
                    'w' => mode |= 2,
                    'r' => mode |= 4,
                    _ => {
                        return Err(format!(
                            "unexpected character '{c}'. excepted one of 'r', 'w' or 'x'"
                        ))
                    }
                };
            }
            Ok(mode)
        }

        #[inline]
        fn parse_alphabetic_mode(
            t: char,
            chars: impl Iterator<Item = char>,
            who: Who,
        ) -> Result<Mode, <Mode as FromStr>::Err> {
            match t {
                '+' => Ok(Mode::Plus(who, parse_mode(chars)?)),
                '-' => Ok(Mode::Minus(who, parse_mode(chars)?)),
                '=' => Ok(Mode::Equal(who, parse_mode(chars)?)),
                m => Err(format!(
                    "unexpected character '{m}'. excepted one of '+', '-' or '='"
                )),
            }
        }
        if s.is_empty() {
            return Err("mode must not be empty".into());
        }
        if s.chars().all(|c| c.is_ascii_digit()) {
            return if s.len() == 3 {
                u16::from_str_radix(s, 8)
                    .map(Self::Numeric)
                    .map_err(|e| e.to_string())
            } else {
                Err(format!("invalid mode length {}", s.len()))
            };
        }
        let mut who = Who::empty();
        for (idx, c) in s.chars().enumerate() {
            match c {
                'u' => who |= Who::User,
                'g' => who |= Who::Group,
                'o' => who |= Who::Other,
                'a' => who |= Who::All,
                t @ ('+' | '-' | '=') => {
                    return parse_alphabetic_mode(
                        t,
                        s.chars().skip(idx + 1),
                        if idx == 0 { Who::All } else { who },
                    )
                }
                first => return Err(format!("unexpected character '{first}'")),
            }
        }
        Err("mode must not be empty".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_digit_mode() {
        assert_eq!(Mode::from_str("755").unwrap(), Mode::Numeric(0o755));
        assert_eq!(Mode::from_str("000").unwrap(), Mode::Numeric(0o000));
    }

    #[test]
    fn parse_alphabetic_mode() {
        assert_eq!(Mode::from_str("=rwx").unwrap(), Mode::Equal(Who::All, 0o7),);
        assert_eq!(Mode::from_str("=rw").unwrap(), Mode::Equal(Who::All, 0o6),);
        assert_eq!(Mode::from_str("+x").unwrap(), Mode::Plus(Who::All, 0o1));
        assert_eq!(Mode::from_str("-w").unwrap(), Mode::Minus(Who::All, 0o2));
    }

    #[test]
    fn parse_alphabetic_mode_with_user() {
        assert_eq!(
            Mode::from_str("u=rwx").unwrap(),
            Mode::Equal(Who::User, 0o7),
        );
        assert_eq!(
            Mode::from_str("g=rw").unwrap(),
            Mode::Equal(Who::Group, 0o6),
        );
        assert_eq!(Mode::from_str("o+x").unwrap(), Mode::Plus(Who::Other, 0o1),);
        assert_eq!(Mode::from_str("a-w").unwrap(), Mode::Minus(Who::All, 0o2),);
        assert_eq!(
            Mode::from_str("ug+x").unwrap(),
            Mode::Plus(Who::User | Who::Group, 0o1),
        );
    }

    #[test]
    fn parse_mode_from_str_symbol_without_mode() {
        assert_eq!(Mode::from_str("u=").unwrap(), Mode::Equal(Who::User, 0));
        assert_eq!(Mode::from_str("g+").unwrap(), Mode::Plus(Who::Group, 0));
        assert_eq!(Mode::from_str("o-").unwrap(), Mode::Minus(Who::Other, 0));
    }

    #[test]
    fn parse_mode_from_str_multiple_targets_symbol_without_mode() {
        assert_eq!(
            Mode::from_str("ug=").unwrap(),
            Mode::Equal(Who::User | Who::Group, 0)
        );
    }

    #[test]
    fn parse_mode_from_str_no_target_before_symbol() {
        assert_eq!(Mode::from_str("=rwx").unwrap(), Mode::Equal(Who::All, 0o7));
        assert_eq!(Mode::from_str("+x").unwrap(), Mode::Plus(Who::All, 0o1));
        assert_eq!(Mode::from_str("-w").unwrap(), Mode::Minus(Who::All, 0o2));
    }

    #[test]
    fn parse_mode_from_str_multiple_targets() {
        assert_eq!(
            Mode::from_str("ugo=rw").unwrap(),
            Mode::Equal(Who::User | Who::Group | Who::Other, 0o6)
        );
        assert_eq!(
            Mode::from_str("ug+x").unwrap(),
            Mode::Plus(Who::User | Who::Group, 0o1)
        );
    }

    #[test]
    fn parse_mode_from_str_all_mixed_with_targets() {
        assert_eq!(Mode::from_str("au=rw").unwrap(), Mode::Equal(Who::All, 0o6));
    }

    #[test]
    fn parse_mode_from_str_empty_string() {
        assert!(Mode::from_str("").is_err());
    }

    #[test]
    fn parse_mode_from_str_invalid_digit_length() {
        assert!(Mode::from_str("77").is_err());
        assert!(Mode::from_str("7777").is_err());
    }

    #[test]
    fn parse_mode_from_str_non_digit_string() {
        assert!(Mode::from_str("abc").is_err());
    }

    #[test]
    fn parse_mode_from_str_invalid_symbol() {
        assert!(Mode::from_str("u?rw").is_err());
        assert!(Mode::from_str("u@rw").is_err());
    }

    #[test]
    fn parse_mode_from_str_invalid_target() {
        assert!(Mode::from_str("z=rw").is_err());
    }

    #[test]
    fn parse_mode_from_str_double_symbol() {
        assert!(Mode::from_str("u==rw").is_err());
        assert!(Mode::from_str("u++x").is_err());
    }

    #[test]
    fn parse_mode_from_str_invalid_char_after_symbol() {
        assert!(Mode::from_str("u=rwa").is_err());
    }

    #[test]
    fn mode_apply_to() {
        assert_eq!(Mode::from_str("755").unwrap().apply_to(0o764), 0o755);
        assert_eq!(Mode::from_str("+x").unwrap().apply_to(0o664), 0o775);
        assert_eq!(Mode::from_str("o+r").unwrap().apply_to(0o600), 0o604);
        assert_eq!(Mode::from_str("u-r").unwrap().apply_to(0o600), 0o200);
        assert_eq!(Mode::from_str("g=rw").unwrap().apply_to(0o777), 0o767);
        assert_eq!(Mode::from_str("u=rw").unwrap().apply_to(0o000), 0o600);
        assert_eq!(Mode::from_str("go-x").unwrap().apply_to(0o777), 0o766);
        assert_eq!(Mode::from_str("go=r").unwrap().apply_to(0o777), 0o744);
    }

    #[test]
    fn who_apply_to_user_only() {
        assert_eq!(Who::User.apply_to(0o7), 0o700);
    }

    #[test]
    fn who_apply_to_group_only() {
        assert_eq!(Who::Group.apply_to(0o7), 0o070);
    }

    #[test]
    fn who_apply_to_other_only() {
        assert_eq!(Who::Other.apply_to(0o7), 0o007);
    }

    #[test]
    fn who_apply_to_user_group() {
        assert_eq!((Who::User | Who::Group).apply_to(0o7), 0o770);
    }

    #[test]
    fn who_apply_to_user_other() {
        assert_eq!((Who::User | Who::Other).apply_to(0o7), 0o707);
    }

    #[test]
    fn who_apply_to_group_other() {
        assert_eq!((Who::Group | Who::Other).apply_to(0o7), 0o077);
    }

    #[test]
    fn who_apply_to_all() {
        assert_eq!(Who::All.apply_to(0o7), 0o777);
    }

    #[test]
    fn who_apply_to_empty() {
        assert_eq!(Who::empty().apply_to(0o7), 0);
    }

    #[test]
    fn who_apply_to_zero() {
        assert_eq!(Who::All.apply_to(0), 0);
    }

    #[test]
    fn mode_apply_to_num() {
        assert_eq!(Mode::Numeric(0o123).apply_to(0o777), 0o123);
    }

    #[test]
    fn mode_apply_to_equal_user_only() {
        assert_eq!(Mode::Equal(Who::User, 0o7).apply_to(0o654), 0o754);
    }

    #[test]
    fn mode_apply_to_equal_group_only() {
        assert_eq!(Mode::Equal(Who::Group, 0o6).apply_to(0o754), 0o764);
    }

    #[test]
    fn mode_apply_to_equal_other_only() {
        assert_eq!(Mode::Equal(Who::Other, 0o5).apply_to(0o764), 0o765);
    }

    #[test]
    fn mode_apply_to_equal_all() {
        assert_eq!(Mode::Equal(Who::All, 0o0).apply_to(0o777), 0o000);
    }

    #[test]
    fn mode_apply_to_equal_multiple_targets() {
        assert_eq!(
            Mode::Equal(Who::User | Who::Group, 0o4).apply_to(0o777),
            0o447
        );
    }

    #[test]
    fn mode_apply_to_plus_user_only() {
        assert_eq!(Mode::Plus(Who::User, 0o1).apply_to(0o600), 0o700);
    }

    #[test]
    fn mode_apply_to_plus_group_only() {
        assert_eq!(Mode::Plus(Who::Group, 0o2).apply_to(0o640), 0o660);
    }

    #[test]
    fn mode_apply_to_plus_other_only() {
        assert_eq!(Mode::Plus(Who::Other, 0o4).apply_to(0o600), 0o604);
    }

    #[test]
    fn mode_apply_to_plus_all() {
        assert_eq!(Mode::Plus(Who::All, 0o1).apply_to(0o660), 0o771);
    }

    #[test]
    fn mode_apply_to_plus_zero() {
        assert_eq!(Mode::Plus(Who::All, 0).apply_to(0o777), 0o777);
    }

    #[test]
    fn mode_apply_to_minus_user_only() {
        assert_eq!(Mode::Minus(Who::User, 0o4).apply_to(0o744), 0o344);
    }

    #[test]
    fn mode_apply_to_minus_group_only() {
        assert_eq!(Mode::Minus(Who::Group, 0o2).apply_to(0o762), 0o742);
    }

    #[test]
    fn mode_apply_to_minus_other_only() {
        assert_eq!(Mode::Minus(Who::Other, 0o1).apply_to(0o701), 0o700);
    }

    #[test]
    fn mode_apply_to_minus_all() {
        assert_eq!(Mode::Minus(Who::All, 0o7).apply_to(0o777), 0o000);
    }

    #[test]
    fn mode_apply_to_minus_zero() {
        assert_eq!(Mode::Minus(Who::All, 0).apply_to(0o777), 0o777);
    }

    #[test]
    fn mode_apply_to_boundary_all_bits_set() {
        assert_eq!(Mode::Plus(Who::All, 0o7).apply_to(0o777), 0o777);
    }

    #[test]
    fn mode_apply_to_boundary_all_bits_cleared() {
        assert_eq!(Mode::Minus(Who::All, 0o7).apply_to(0o000), 0o000);
    }
}
