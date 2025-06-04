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
use nom::{
    branch::alt,
    character::complete::char,
    combinator::{map, opt},
    multi::{many0, many1, separated_list1},
    Parser as _,
};
use pna::NormalEntry;
use std::{io, ops::BitOr, path::PathBuf, str::FromStr};

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

    #[inline]
    fn parse_from(s: &str) -> nom::IResult<&str, Who> {
        alt((
            map(char('a'), |_| Self::All),
            map(char('u'), |_| Self::User),
            map(char('g'), |_| Self::Group),
            map(char('o'), |_| Self::Other),
        ))
        .parse(s)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Action {
    Equal(u8),
    Plus(u8),
    Minus(u8),
}

impl Action {
    #[inline]
    fn parse_from(s: &str) -> nom::IResult<&str, Self> {
        fn op(s: &str) -> nom::IResult<&str, Action> {
            alt((
                map(char('+'), |_| Action::Plus(0)),
                map(char('-'), |_| Action::Minus(0)),
                map(char('='), |_| Action::Equal(0)),
            ))
            .parse(s)
        }
        fn perm(s: &str) -> nom::IResult<&str, u8> {
            alt((
                map(char('r'), |_| 0o4),
                map(char('w'), |_| 0o2),
                map(char('x'), |_| 0o1),
            ))
            .parse(s)
        }

        map((op, many0(perm)), |(op, perms)| match op {
            Action::Equal(m) => Action::Equal(perms.into_iter().fold(m, BitOr::bitor)),
            Action::Plus(m) => Action::Plus(perms.into_iter().fold(m, BitOr::bitor)),
            Action::Minus(m) => Action::Minus(perms.into_iter().fold(m, BitOr::bitor)),
        })
        .parse(s)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ModeClause {
    who: Who,
    actions: Vec<Action>,
}

impl ModeClause {
    #[inline]
    fn parse_from(s: &str) -> nom::IResult<&str, Self> {
        map(
            (opt(many1(Who::parse_from)), many1(Action::parse_from)),
            |(who, actions)| ModeClause {
                who: who
                    .map(|w| w.into_iter().fold(Who::empty(), BitOr::bitor))
                    .unwrap_or(Who::All),
                actions,
            },
        )
        .parse(s)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Mode {
    Numeric(u16),
    Clause(Vec<ModeClause>),
}

impl Mode {
    const OWNER_MASK: u16 = 0o700;
    const GROUP_MASK: u16 = 0o070;
    const OTHER_MASK: u16 = 0o007;
    #[inline]
    pub(crate) fn apply_to(&self, mut mode: u16) -> u16 {
        match self {
            Mode::Numeric(mode) => *mode,
            Mode::Clause(clauses) => {
                for ModeClause { who, actions } in clauses {
                    for action in actions {
                        match action {
                            Action::Equal(m) => {
                                let owner_mode = if who.contains(Who::User) {
                                    Who::User.apply_to(*m as u16)
                                } else {
                                    mode & Self::OWNER_MASK
                                };
                                let group_mode = if who.contains(Who::Group) {
                                    Who::Group.apply_to(*m as u16)
                                } else {
                                    mode & Self::GROUP_MASK
                                };
                                let other_mode = if who.contains(Who::Other) {
                                    Who::Other.apply_to(*m as u16)
                                } else {
                                    mode & Self::OTHER_MASK
                                };
                                mode = owner_mode | group_mode | other_mode
                            }
                            Action::Plus(m) => mode |= who.apply_to(*m as u16),
                            Action::Minus(m) => mode &= !who.apply_to(*m as u16),
                        }
                    }
                }
                mode
            }
        }
    }
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.chars().all(|c| c.is_ascii_digit()) {
            return if s.len() == 3 {
                u16::from_str_radix(s, 8)
                    .map(Self::Numeric)
                    .map_err(|e| e.to_string())
            } else {
                Err(format!("Invalid mode length: {}", s.len()))
            };
        }
        separated_list1(char(','), ModeClause::parse_from)
            .parse_complete(s)
            .map_err(|e| e.to_string())
            .and_then(|(remain, mode)| {
                if remain.is_empty() {
                    Ok(Mode::Clause(mode))
                } else {
                    Err(format!("Invalid file mode: {s}"))
                }
            })
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
        assert_eq!(
            Mode::from_str("=rwx").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Equal(0o7)]
            }])
        );
        assert_eq!(
            Mode::from_str("=rw").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Equal(0o6)]
            }])
        );
        assert_eq!(
            Mode::from_str("+x").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Plus(0o1)]
            }])
        );
        assert_eq!(
            Mode::from_str("-w").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Minus(0o2)]
            }])
        );
    }

    #[test]
    fn parse_alphabetic_mode_with_user() {
        assert_eq!(
            Mode::from_str("u=rwx").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o7)]
            }])
        );
        assert_eq!(
            Mode::from_str("g=rw").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::Group,
                actions: vec![Action::Equal(0o6)]
            }])
        );
        assert_eq!(
            Mode::from_str("o+x").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::Other,
                actions: vec![Action::Plus(0o1)]
            }])
        );
        assert_eq!(
            Mode::from_str("a-w").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Minus(0o2)]
            }])
        );
    }

    #[test]
    fn parse_mode_from_str_symbol_without_mode() {
        assert_eq!(
            Mode::from_str("u=").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0)]
            }])
        );
        assert_eq!(
            Mode::from_str("g+").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::Group,
                actions: vec![Action::Plus(0)]
            }])
        );
        assert_eq!(
            Mode::from_str("o-").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::Other,
                actions: vec![Action::Minus(0)]
            }])
        );
    }

    #[test]
    fn parse_mode_from_str_multiple_targets_symbol_without_mode() {
        assert_eq!(
            Mode::from_str("ug=").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User | Who::Group,
                actions: vec![Action::Equal(0)]
            }])
        );
    }

    #[test]
    fn parse_mode_from_str_no_target_before_symbol() {
        assert_eq!(
            Mode::from_str("=rwx").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Equal(0o7)]
            }])
        );
        assert_eq!(
            Mode::from_str("+x").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Plus(0o1)]
            }])
        );
        assert_eq!(
            Mode::from_str("-w").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Minus(0o2)]
            }])
        );
    }

    #[test]
    fn parse_mode_from_str_multiple_targets() {
        assert_eq!(
            Mode::from_str("ugo=rw").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User | Who::Group | Who::Other,
                actions: vec![Action::Equal(0o6)]
            }])
        );
        assert_eq!(
            Mode::from_str("ug+x").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User | Who::Group,
                actions: vec![Action::Plus(0o1)]
            }])
        );
    }

    #[test]
    fn parse_mode_from_str_all_mixed_with_targets() {
        assert_eq!(
            Mode::from_str("au=rw").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Equal(0o6)]
            }])
        );
    }

    #[test]
    fn parse_mode_from_str_multiple_clauses() {
        assert_eq!(
            Mode::from_str("u=rwx,g=rx,o=r").unwrap(),
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o7)]
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Equal(0o5)]
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o4)]
                }
            ])
        );
    }

    #[test]
    fn parse_mode_from_str_multiple_actions() {
        assert_eq!(
            Mode::from_str("u=rwx,g+rx,o-r").unwrap(),
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o7)]
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Plus(0o5)]
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Minus(0o4)]
                }
            ])
        );
    }

    #[test]
    fn parse_mode_from_str_complex_combinations() {
        assert_eq!(
            Mode::from_str("ug=rwx,o=rx").unwrap(),
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User | Who::Group,
                    actions: vec![Action::Equal(0o7)]
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o5)]
                }
            ])
        );
    }

    #[test]
    fn parse_mode_from_str_empty_perms() {
        assert_eq!(
            Mode::from_str("u=,g=,o=").unwrap(),
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0)]
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Equal(0)]
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0)]
                }
            ])
        );
    }

    #[test]
    fn parse_mode_from_str_all_perm_combinations() {
        assert_eq!(
            Mode::from_str("u=rwx,g=rw,o=r").unwrap(),
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o7)]
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Equal(0o6)]
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o4)]
                }
            ])
        );
    }

    #[test]
    fn parse_mode_from_str_invalid_multiple_clauses() {
        assert!(Mode::from_str("u=rwx,,g=rx").is_err());
        assert!(Mode::from_str("u=rwx,g=rx,").is_err());
        assert!(Mode::from_str(",u=rwx,g=rx").is_err());
    }

    #[test]
    fn parse_mode_from_str_multiple_actions_in_single_clause() {
        assert_eq!(
            Mode::from_str("u=rwx+rx").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o7), Action::Plus(0o5)],
            }])
        );
        assert_eq!(
            Mode::from_str("u=rwx-rx").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o7), Action::Minus(0o5)],
            }])
        );
        assert_eq!(
            Mode::from_str("u+rwx=rx").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Plus(0o7), Action::Equal(0o5)],
            }])
        );
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
        assert_eq!(
            Mode::from_str("u==rw").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o0), Action::Equal(0o6)],
            }])
        );
        assert_eq!(
            Mode::from_str("u++x").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Plus(0o0), Action::Plus(0o1)],
            }])
        );
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
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o7)]
            }])
            .apply_to(0o654),
            0o754
        );
    }

    #[test]
    fn mode_apply_to_equal_group_only() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Group,
                actions: vec![Action::Equal(0o6)]
            }])
            .apply_to(0o754),
            0o764
        );
    }

    #[test]
    fn mode_apply_to_equal_other_only() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Other,
                actions: vec![Action::Equal(0o5)]
            }])
            .apply_to(0o764),
            0o765
        );
    }

    #[test]
    fn mode_apply_to_equal_all() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Equal(0o0)]
            }])
            .apply_to(0o777),
            0o000
        );
    }

    #[test]
    fn mode_apply_to_equal_multiple_targets() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User | Who::Group,
                actions: vec![Action::Equal(0o4)]
            }])
            .apply_to(0o777),
            0o447
        );
    }

    #[test]
    fn mode_apply_to_plus_user_only() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Plus(0o1)]
            }])
            .apply_to(0o600),
            0o700
        );
    }

    #[test]
    fn mode_apply_to_plus_group_only() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Group,
                actions: vec![Action::Plus(0o2)]
            }])
            .apply_to(0o640),
            0o660
        );
    }

    #[test]
    fn mode_apply_to_plus_other_only() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Other,
                actions: vec![Action::Plus(0o4)]
            }])
            .apply_to(0o600),
            0o604
        );
    }

    #[test]
    fn mode_apply_to_plus_all() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Plus(0o1)]
            }])
            .apply_to(0o660),
            0o771
        );
    }

    #[test]
    fn mode_apply_to_plus_zero() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Plus(0)]
            }])
            .apply_to(0o777),
            0o777
        );
    }

    #[test]
    fn mode_apply_to_minus_user_only() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Minus(0o4)]
            }])
            .apply_to(0o744),
            0o344
        );
    }

    #[test]
    fn mode_apply_to_minus_group_only() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Group,
                actions: vec![Action::Minus(0o2)]
            }])
            .apply_to(0o762),
            0o742
        );
    }

    #[test]
    fn mode_apply_to_minus_other_only() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Other,
                actions: vec![Action::Minus(0o1)]
            }])
            .apply_to(0o701),
            0o700
        );
    }

    #[test]
    fn mode_apply_to_minus_all() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Minus(0o7)]
            }])
            .apply_to(0o777),
            0o000
        );
    }

    #[test]
    fn mode_apply_to_minus_zero() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Minus(0)]
            }])
            .apply_to(0o777),
            0o777
        );
    }

    #[test]
    fn mode_apply_to_boundary_all_bits_set() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Plus(0o7)]
            }])
            .apply_to(0o777),
            0o777
        );
    }

    #[test]
    fn mode_apply_to_boundary_all_bits_cleared() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Minus(0o7)]
            }])
            .apply_to(0o000),
            0o000
        );
    }

    #[test]
    fn mode_apply_to_multiple_actions_in_single_clause() {
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o7), Action::Plus(0o5)],
            }])
            .apply_to(0o000),
            0o700
        );
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o7), Action::Minus(0o5)],
            }])
            .apply_to(0o777),
            0o277
        );
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Plus(0o7), Action::Equal(0o5)],
            }])
            .apply_to(0o000),
            0o500
        );
    }

    #[test]
    fn mode_apply_to_multiple_clauses() {
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o7)],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Equal(0o5)],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o4)],
                }
            ])
            .apply_to(0o000),
            0o754
        );
    }

    #[test]
    fn mode_apply_to_complex_combinations() {
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User | Who::Group,
                    actions: vec![Action::Equal(0o7)],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o5)],
                }
            ])
            .apply_to(0o000),
            0o775
        );
    }

    #[test]
    fn mode_apply_to_empty_perms() {
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0)],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Equal(0)],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0)],
                }
            ])
            .apply_to(0o777),
            0o000
        );
    }

    #[test]
    fn mode_apply_to_all_perm_combinations() {
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o7)],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Equal(0o6)],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o4)],
                }
            ])
            .apply_to(0o000),
            0o764
        );
    }

    #[test]
    fn mode_apply_to_multiple_clauses_with_plus_minus() {
        // Plus and Minus combinations
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Plus(0o7)],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Minus(0o5)],
                }
            ])
            .apply_to(0o000),
            0o700
        );

        // Cumulative effect of multiple Plus actions
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Plus(0o4)],
                },
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Plus(0o2)],
                }
            ])
            .apply_to(0o000),
            0o600
        );

        // Cumulative effect of multiple Minus actions
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Minus(0o4)],
                },
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Minus(0o2)],
                }
            ])
            .apply_to(0o777),
            0o177
        );
    }

    #[test]
    fn mode_apply_to_multiple_clauses_with_mixed_actions() {
        // Mix of Equal, Plus, and Minus actions
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o7)],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Plus(0o5)],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Minus(0o4)],
                }
            ])
            .apply_to(0o000),
            0o750
        );

        // Different actions for the same who
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o4)],
                },
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Plus(0o2)],
                }
            ])
            .apply_to(0o000),
            0o600
        );
    }

    #[test]
    fn mode_apply_to_multiple_clauses_with_overlapping_who() {
        // Overlapping who specifications
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User | Who::Group,
                    actions: vec![Action::Equal(0o7)],
                },
                ModeClause {
                    who: Who::Group | Who::Other,
                    actions: vec![Action::Plus(0o5)],
                }
            ])
            .apply_to(0o000),
            0o775
        );

        // Multiple actions for multiple who
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User | Who::Group,
                    actions: vec![Action::Equal(0o7)],
                },
                ModeClause {
                    who: Who::Group | Who::Other,
                    actions: vec![Action::Minus(0o5)],
                }
            ])
            .apply_to(0o777),
            0o722
        );
    }

    #[test]
    fn mode_apply_to_multiple_clauses_with_action_order() {
        // Verify action application order
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Plus(0o7)],
                },
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o5)],
                }
            ])
            .apply_to(0o000),
            0o500
        );

        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o7)],
                },
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Minus(0o5)],
                }
            ])
            .apply_to(0o000),
            0o200
        );
    }

    #[test]
    fn mode_apply_to_with_intermediate_permissions() {
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o7)],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Plus(0o5)],
                }
            ])
            .apply_to(0o123),
            0o773
        );
    }

    #[test]
    fn mode_apply_to_with_partial_permissions() {
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Minus(0o4)],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Plus(0o2)],
                }
            ])
            .apply_to(0o421),
            0o021
        );
    }

    #[test]
    fn mode_apply_to_with_specific_bit_patterns() {
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User | Who::Group,
                    actions: vec![Action::Equal(0o4)],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Plus(0o1)],
                }
            ])
            .apply_to(0o242),
            0o443
        );
    }

    #[test]
    fn mode_apply_to_with_irregular_permissions() {
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o6)],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Minus(0o2)],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Plus(0o1)],
                }
            ])
            .apply_to(0o135),
            0o615
        );
    }

    #[test]
    fn mode_apply_to_with_uniform_permissions() {
        // Test applying changes to a mode where all who have the same permissions
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User | Who::Group,
                    actions: vec![Action::Minus(0o3)],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o4)],
                }
            ])
            .apply_to(0o666),
            0o444
        );
    }

    #[test]
    fn mode_apply_to_with_sequential_actions() {
        // Test applying multiple actions to the same who in sequence
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o4)],
                },
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Plus(0o2)],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Minus(0o1)],
                }
            ])
            .apply_to(0o531),
            0o621
        );
    }

    #[test]
    fn mode_apply_to_with_overlapping_who() {
        // Test applying changes when who specifications overlap
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User | Who::Group,
                    actions: vec![Action::Equal(0o5)],
                },
                ModeClause {
                    who: Who::Group | Who::Other,
                    actions: vec![Action::Plus(0o2)],
                }
            ])
            .apply_to(0o246),
            0o576
        );
    }

    #[test]
    fn mode_apply_to_with_special_permission_patterns() {
        // Test with execute-only permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o1)],
            }])
            .apply_to(0o777),
            0o177
        );

        // Test with write-only permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Group,
                actions: vec![Action::Equal(0o2)],
            }])
            .apply_to(0o777),
            0o727
        );

        // Test with read-only permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Other,
                actions: vec![Action::Equal(0o4)],
            }])
            .apply_to(0o777),
            0o774
        );

        // Test with execute and write permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o3)],
            }])
            .apply_to(0o777),
            0o377
        );

        // Test with execute and read permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Group,
                actions: vec![Action::Equal(0o5)],
            }])
            .apply_to(0o777),
            0o757
        );

        // Test with write and read permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Other,
                actions: vec![Action::Equal(0o6)],
            }])
            .apply_to(0o777),
            0o776
        );
    }

    #[test]
    fn mode_apply_to_with_boundary_permissions() {
        // Test from maximum permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User | Who::Group | Who::Other,
                actions: vec![Action::Minus(0o7)],
            }])
            .apply_to(0o777),
            0o000
        );

        // Test from minimum permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User | Who::Group | Who::Other,
                actions: vec![Action::Plus(0o7)],
            }])
            .apply_to(0o000),
            0o777
        );

        // Test with maximum permissions for specific who
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o7)],
            }])
            .apply_to(0o000),
            0o700
        );
    }

    #[test]
    fn mode_apply_to_with_combined_actions() {
        // Test multiple Equal actions for the same who
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o4), Action::Equal(0o2)],
            }])
            .apply_to(0o777),
            0o277
        );

        // Test multiple Plus actions for the same who
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Group,
                actions: vec![Action::Plus(0o4), Action::Plus(0o2)],
            }])
            .apply_to(0o000),
            0o060
        );

        // Test multiple Minus actions for the same who
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Other,
                actions: vec![Action::Minus(0o4), Action::Minus(0o2)],
            }])
            .apply_to(0o777),
            0o771
        );

        // Test combination of Plus and Minus actions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Plus(0o4), Action::Minus(0o2)],
            }])
            .apply_to(0o000),
            0o400
        );
    }

    #[test]
    fn mode_apply_to_with_who_combinations() {
        // Test all possible who combinations with different actions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User | Who::Group | Who::Other,
                actions: vec![Action::Equal(0o7)],
            }])
            .apply_to(0o000),
            0o777
        );

        // Test User+Group with one action and Group+Other with another
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User | Who::Group,
                    actions: vec![Action::Equal(0o5)],
                },
                ModeClause {
                    who: Who::Group | Who::Other,
                    actions: vec![Action::Equal(0o3)],
                }
            ])
            .apply_to(0o000),
            0o533
        );

        // Test overlapping who specifications with different actions
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User | Who::Group,
                    actions: vec![Action::Plus(0o4)],
                },
                ModeClause {
                    who: Who::Group | Who::Other,
                    actions: vec![Action::Minus(0o2)],
                }
            ])
            .apply_to(0o000),
            0o440
        );
    }
}
