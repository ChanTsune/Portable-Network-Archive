use crate::{
    cli::{PasswordArgs, SolidEntriesTransformStrategy, SolidEntriesTransformStrategyArgs},
    command::{
        Command, ask_password,
        core::{
            SplitArchiveReader, TransformStrategyKeepSolid, TransformStrategyUnSolid,
            collect_split_archives,
        },
    },
    utils::{GlobPatterns, PathPartExt, env::NamedTempFile},
};
use bitflags::bitflags;
use clap::{Parser, ValueHint};
use nom::{
    Parser as _,
    branch::alt,
    character::complete::char,
    combinator::{map, opt},
    multi::{many0, many1, separated_list1},
};
use pna::NormalEntry;
use std::{ops::BitOr, path::PathBuf, str::FromStr};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ChmodCommand {
    #[arg(short = 'f', long = "file", value_hint = ValueHint::FilePath)]
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
    fn execute(self, _ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        archive_chmod(self)
    }
}

#[hooq::hooq(anyhow)]
fn archive_chmod(args: ChmodCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    if args.files.is_empty() {
        return Ok(());
    }
    let mut globs = GlobPatterns::new(args.files.iter().map(|p| p.as_str()))?;

    let mut source = SplitArchiveReader::new(collect_split_archives(&args.archive)?)?;

    let output_path = args.archive.remove_part();
    let mut temp_file =
        NamedTempFile::new(|| output_path.parent().unwrap_or_else(|| ".".as_ref()))?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => source.transform_entries(
            temp_file.as_file_mut(),
            password.as_deref(),
            #[hooq::skip_all]
            |entry| {
                let entry = entry?;
                if globs.matches_any(entry.name()) {
                    Ok(Some(transform_entry(entry, &args.mode)))
                } else {
                    Ok(Some(entry))
                }
            },
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => source.transform_entries(
            temp_file.as_file_mut(),
            password.as_deref(),
            #[hooq::skip_all]
            |entry| {
                let entry = entry?;
                if globs.matches_any(entry.name()) {
                    Ok(Some(transform_entry(entry, &args.mode)))
                } else {
                    Ok(Some(entry))
                }
            },
            TransformStrategyKeepSolid,
        ),
    }?;

    drop(source);

    temp_file.persist(output_path)?;

    globs.ensure_all_matched()?;
    Ok(())
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
    /// Expand 3-bit permission value to positions specified by this Who mask.
    #[inline]
    const fn to_permission_bits(self, n: u16) -> u16 {
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

    /// Extract 3-bit permission values from positions specified by this Who mask.
    #[inline]
    const fn extract_bits(self, mode: u16) -> u16 {
        let mut result = 0;
        if self.contains(Who::User) {
            result |= (mode >> 6) & 0o7;
        }
        if self.contains(Who::Group) {
            result |= (mode >> 3) & 0o7;
        }
        if self.contains(Who::Other) {
            result |= mode & 0o7;
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

/// Permission bits that can be either literal (r, w, x) or copied from another class (u, g, o).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct PermBits {
    /// Direct permission bits (r=4, w=2, x=1)
    literal: u8,
    /// Sources to copy from (resolved at apply time)
    copy_from: Who,
}

impl Default for PermBits {
    #[inline]
    fn default() -> Self {
        Self {
            literal: 0,
            copy_from: Who::empty(),
        }
    }
}

impl From<u8> for PermBits {
    #[inline]
    fn from(literal: u8) -> Self {
        Self {
            literal,
            copy_from: Who::empty(),
        }
    }
}

impl PermBits {
    /// Create a PermBits that copies from the given source(s).
    const fn copy_from(source: Who) -> Self {
        Self {
            literal: 0,
            copy_from: source,
        }
    }

    /// Resolve copy sources against current mode to get final permission bits.
    #[must_use]
    #[inline]
    const fn resolve(self, mode: u16) -> u16 {
        self.literal as u16 | self.copy_from.extract_bits(mode)
    }
}

impl BitOr for PermBits {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self {
            literal: self.literal | rhs.literal,
            copy_from: self.copy_from | rhs.copy_from,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Action {
    Equal(PermBits),
    Plus(PermBits),
    Minus(PermBits),
}

impl Action {
    #[inline]
    fn parse_from(s: &str) -> nom::IResult<&str, Self> {
        #[derive(Copy, Clone)]
        enum Op {
            Plus,
            Minus,
            Equal,
        }

        fn op(s: &str) -> nom::IResult<&str, Op> {
            alt((
                map(char('+'), |_| Op::Plus),
                map(char('-'), |_| Op::Minus),
                map(char('='), |_| Op::Equal),
            ))
            .parse(s)
        }

        fn perm(s: &str) -> nom::IResult<&str, PermBits> {
            alt((
                // Literal permission bits
                map(char('r'), |_| 0o4.into()),
                map(char('w'), |_| 0o2.into()),
                map(char('x'), |_| 0o1.into()),
                // Copy sources
                map(char('u'), |_| PermBits::copy_from(Who::User)),
                map(char('g'), |_| PermBits::copy_from(Who::Group)),
                map(char('o'), |_| PermBits::copy_from(Who::Other)),
            ))
            .parse(s)
        }

        map((op, many0(perm)), |(op, perms)| {
            let bits = perms.into_iter().fold(PermBits::default(), BitOr::bitor);
            match op {
                Op::Plus => Action::Plus(bits),
                Op::Minus => Action::Minus(bits),
                Op::Equal => Action::Equal(bits),
            }
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
    #[inline]
    pub(crate) fn apply_to(&self, mut mode: u16) -> u16 {
        match self {
            Mode::Numeric(mode) => *mode,
            Mode::Clause(clauses) => {
                for ModeClause { who, actions } in clauses {
                    for action in actions {
                        match action {
                            Action::Equal(bits) => {
                                // Resolve copy sources against current mode
                                let m = bits.resolve(mode);
                                let mask = who.to_permission_bits(0o7);
                                mode = (mode & !mask) | who.to_permission_bits(m);
                            }
                            Action::Plus(bits) => {
                                let m = bits.resolve(mode);
                                mode |= who.to_permission_bits(m)
                            }
                            Action::Minus(bits) => {
                                let m = bits.resolve(mode);
                                mode &= !who.to_permission_bits(m)
                            }
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
                actions: vec![Action::Equal(0o7.into())]
            }])
        );
        assert_eq!(
            Mode::from_str("=rw").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Equal(0o6.into())]
            }])
        );
        assert_eq!(
            Mode::from_str("+x").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Plus(0o1.into())]
            }])
        );
        assert_eq!(
            Mode::from_str("-w").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Minus(0o2.into())]
            }])
        );
    }

    #[test]
    fn parse_alphabetic_mode_with_user() {
        assert_eq!(
            Mode::from_str("u=rwx").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o7.into())]
            }])
        );
        assert_eq!(
            Mode::from_str("g=rw").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::Group,
                actions: vec![Action::Equal(0o6.into())]
            }])
        );
        assert_eq!(
            Mode::from_str("o+x").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::Other,
                actions: vec![Action::Plus(0o1.into())]
            }])
        );
        assert_eq!(
            Mode::from_str("a-w").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Minus(0o2.into())]
            }])
        );
    }

    #[test]
    fn parse_mode_from_str_symbol_without_mode() {
        assert_eq!(
            Mode::from_str("u=").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0.into())]
            }])
        );
        assert_eq!(
            Mode::from_str("g+").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::Group,
                actions: vec![Action::Plus(0.into())]
            }])
        );
        assert_eq!(
            Mode::from_str("o-").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::Other,
                actions: vec![Action::Minus(0.into())]
            }])
        );
    }

    #[test]
    fn parse_mode_from_str_multiple_targets_symbol_without_mode() {
        assert_eq!(
            Mode::from_str("ug=").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User | Who::Group,
                actions: vec![Action::Equal(0.into())]
            }])
        );
    }

    #[test]
    fn parse_mode_from_str_no_target_before_symbol() {
        assert_eq!(
            Mode::from_str("=rwx").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Equal(0o7.into())]
            }])
        );
        assert_eq!(
            Mode::from_str("+x").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Plus(0o1.into())]
            }])
        );
        assert_eq!(
            Mode::from_str("-w").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Minus(0o2.into())]
            }])
        );
    }

    #[test]
    fn parse_mode_from_str_multiple_targets() {
        assert_eq!(
            Mode::from_str("ugo=rw").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User | Who::Group | Who::Other,
                actions: vec![Action::Equal(0o6.into())]
            }])
        );
        assert_eq!(
            Mode::from_str("ug+x").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User | Who::Group,
                actions: vec![Action::Plus(0o1.into())]
            }])
        );
    }

    #[test]
    fn parse_mode_from_str_all_mixed_with_targets() {
        assert_eq!(
            Mode::from_str("au=rw").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::All,
                actions: vec![Action::Equal(0o6.into())]
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
                    actions: vec![Action::Equal(0o7.into())]
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Equal(0o5.into())]
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o4.into())]
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
                    actions: vec![Action::Equal(0o7.into())]
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Plus(0o5.into())]
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Minus(0o4.into())]
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
                    actions: vec![Action::Equal(0o7.into())]
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o5.into())]
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
                    actions: vec![Action::Equal(0.into())]
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Equal(0.into())]
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0.into())]
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
                    actions: vec![Action::Equal(0o7.into())]
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Equal(0o6.into())]
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o4.into())]
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
                actions: vec![Action::Equal(0o7.into()), Action::Plus(0o5.into())],
            }])
        );
        assert_eq!(
            Mode::from_str("u=rwx-rx").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o7.into()), Action::Minus(0o5.into())],
            }])
        );
        assert_eq!(
            Mode::from_str("u+rwx=rx").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Plus(0o7.into()), Action::Equal(0o5.into())],
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
                actions: vec![Action::Equal(0o0.into()), Action::Equal(0o6.into())],
            }])
        );
        assert_eq!(
            Mode::from_str("u++x").unwrap(),
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Plus(0o0.into()), Action::Plus(0o1.into())],
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
    fn who_to_permission_bits_user_only() {
        assert_eq!(Who::User.to_permission_bits(0o7), 0o700);
    }

    #[test]
    fn who_to_permission_bits_group_only() {
        assert_eq!(Who::Group.to_permission_bits(0o7), 0o070);
    }

    #[test]
    fn who_to_permission_bits_other_only() {
        assert_eq!(Who::Other.to_permission_bits(0o7), 0o007);
    }

    #[test]
    fn who_to_permission_bits_user_group() {
        assert_eq!((Who::User | Who::Group).to_permission_bits(0o7), 0o770);
    }

    #[test]
    fn who_to_permission_bits_user_other() {
        assert_eq!((Who::User | Who::Other).to_permission_bits(0o7), 0o707);
    }

    #[test]
    fn who_to_permission_bits_group_other() {
        assert_eq!((Who::Group | Who::Other).to_permission_bits(0o7), 0o077);
    }

    #[test]
    fn who_to_permission_bits_all() {
        assert_eq!(Who::All.to_permission_bits(0o7), 0o777);
    }

    #[test]
    fn who_to_permission_bits_empty() {
        assert_eq!(Who::empty().to_permission_bits(0o7), 0);
    }

    #[test]
    fn who_to_permission_bits_zero() {
        assert_eq!(Who::All.to_permission_bits(0), 0);
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
                actions: vec![Action::Equal(0o7.into())]
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
                actions: vec![Action::Equal(0o6.into())]
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
                actions: vec![Action::Equal(0o5.into())]
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
                actions: vec![Action::Equal(0o0.into())]
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
                actions: vec![Action::Equal(0o4.into())]
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
                actions: vec![Action::Plus(0o1.into())]
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
                actions: vec![Action::Plus(0o2.into())]
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
                actions: vec![Action::Plus(0o4.into())]
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
                actions: vec![Action::Plus(0o1.into())]
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
                actions: vec![Action::Plus(0.into())]
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
                actions: vec![Action::Minus(0o4.into())]
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
                actions: vec![Action::Minus(0o2.into())]
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
                actions: vec![Action::Minus(0o1.into())]
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
                actions: vec![Action::Minus(0o7.into())]
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
                actions: vec![Action::Minus(0.into())]
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
                actions: vec![Action::Plus(0o7.into())]
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
                actions: vec![Action::Minus(0o7.into())]
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
                actions: vec![Action::Equal(0o7.into()), Action::Plus(0o5.into())],
            }])
            .apply_to(0o000),
            0o700
        );
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o7.into()), Action::Minus(0o5.into())],
            }])
            .apply_to(0o777),
            0o277
        );
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Plus(0o7.into()), Action::Equal(0o5.into())],
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
                    actions: vec![Action::Equal(0o7.into())],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Equal(0o5.into())],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o4.into())],
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
                    actions: vec![Action::Equal(0o7.into())],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o5.into())],
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
                    actions: vec![Action::Equal(0.into())],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Equal(0.into())],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0.into())],
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
                    actions: vec![Action::Equal(0o7.into())],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Equal(0o6.into())],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o4.into())],
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
                    actions: vec![Action::Plus(0o7.into())],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Minus(0o5.into())],
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
                    actions: vec![Action::Plus(0o4.into())],
                },
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Plus(0o2.into())],
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
                    actions: vec![Action::Minus(0o4.into())],
                },
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Minus(0o2.into())],
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
                    actions: vec![Action::Equal(0o7.into())],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Plus(0o5.into())],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Minus(0o4.into())],
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
                    actions: vec![Action::Equal(0o4.into())],
                },
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Plus(0o2.into())],
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
                    actions: vec![Action::Equal(0o7.into())],
                },
                ModeClause {
                    who: Who::Group | Who::Other,
                    actions: vec![Action::Plus(0o5.into())],
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
                    actions: vec![Action::Equal(0o7.into())],
                },
                ModeClause {
                    who: Who::Group | Who::Other,
                    actions: vec![Action::Minus(0o5.into())],
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
                    actions: vec![Action::Plus(0o7.into())],
                },
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o5.into())],
                }
            ])
            .apply_to(0o000),
            0o500
        );

        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Equal(0o7.into())],
                },
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Minus(0o5.into())],
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
                    actions: vec![Action::Equal(0o7.into())],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Plus(0o5.into())],
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
                    actions: vec![Action::Minus(0o4.into())],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Plus(0o2.into())],
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
                    actions: vec![Action::Equal(0o4.into())],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Plus(0o1.into())],
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
                    actions: vec![Action::Equal(0o6.into())],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Minus(0o2.into())],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Plus(0o1.into())],
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
                    actions: vec![Action::Minus(0o3.into())],
                },
                ModeClause {
                    who: Who::Other,
                    actions: vec![Action::Equal(0o4.into())],
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
                    actions: vec![Action::Equal(0o4.into())],
                },
                ModeClause {
                    who: Who::User,
                    actions: vec![Action::Plus(0o2.into())],
                },
                ModeClause {
                    who: Who::Group,
                    actions: vec![Action::Minus(0o1.into())],
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
                    actions: vec![Action::Equal(0o5.into())],
                },
                ModeClause {
                    who: Who::Group | Who::Other,
                    actions: vec![Action::Plus(0o2.into())],
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
                actions: vec![Action::Equal(0o1.into())],
            }])
            .apply_to(0o777),
            0o177
        );

        // Test with write-only permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Group,
                actions: vec![Action::Equal(0o2.into())],
            }])
            .apply_to(0o777),
            0o727
        );

        // Test with read-only permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Other,
                actions: vec![Action::Equal(0o4.into())],
            }])
            .apply_to(0o777),
            0o774
        );

        // Test with execute and write permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o3.into())],
            }])
            .apply_to(0o777),
            0o377
        );

        // Test with execute and read permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Group,
                actions: vec![Action::Equal(0o5.into())],
            }])
            .apply_to(0o777),
            0o757
        );

        // Test with write and read permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Other,
                actions: vec![Action::Equal(0o6.into())],
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
                actions: vec![Action::Minus(0o7.into())],
            }])
            .apply_to(0o777),
            0o000
        );

        // Test from minimum permissions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User | Who::Group | Who::Other,
                actions: vec![Action::Plus(0o7.into())],
            }])
            .apply_to(0o000),
            0o777
        );

        // Test with maximum permissions for specific who
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Equal(0o7.into())],
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
                actions: vec![Action::Equal(0o4.into()), Action::Equal(0o2.into())],
            }])
            .apply_to(0o777),
            0o277
        );

        // Test multiple Plus actions for the same who
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Group,
                actions: vec![Action::Plus(0o4.into()), Action::Plus(0o2.into())],
            }])
            .apply_to(0o000),
            0o060
        );

        // Test multiple Minus actions for the same who
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::Other,
                actions: vec![Action::Minus(0o4.into()), Action::Minus(0o2.into())],
            }])
            .apply_to(0o777),
            0o771
        );

        // Test combination of Plus and Minus actions
        assert_eq!(
            Mode::Clause(vec![ModeClause {
                who: Who::User,
                actions: vec![Action::Plus(0o4.into()), Action::Minus(0o2.into())],
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
                actions: vec![Action::Equal(0o7.into())],
            }])
            .apply_to(0o000),
            0o777
        );

        // Test User+Group with one action and Group+Other with another
        assert_eq!(
            Mode::Clause(vec![
                ModeClause {
                    who: Who::User | Who::Group,
                    actions: vec![Action::Equal(0o5.into())],
                },
                ModeClause {
                    who: Who::Group | Who::Other,
                    actions: vec![Action::Equal(0o3.into())],
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
                    actions: vec![Action::Plus(0o4.into())],
                },
                ModeClause {
                    who: Who::Group | Who::Other,
                    actions: vec![Action::Minus(0o2.into())],
                }
            ])
            .apply_to(0o000),
            0o440
        );
    }

    #[test]
    fn parse_mode_copy_from_user() {
        // g=u means "set group permissions to whatever user has"
        let mode = Mode::from_str("g=u").unwrap();
        assert!(matches!(mode, Mode::Clause(_)));
        if let Mode::Clause(clauses) = mode {
            assert_eq!(clauses.len(), 1);
            assert_eq!(clauses[0].who, Who::Group);
            assert_eq!(clauses[0].actions.len(), 1);
            if let Action::Equal(bits) = clauses[0].actions[0] {
                assert_eq!(bits.literal, 0);
                assert_eq!(bits.copy_from, Who::User);
            } else {
                panic!("Expected Action::Equal");
            }
        }
    }

    #[test]
    fn parse_mode_copy_from_group() {
        // u=g means "set user permissions to whatever group has"
        let mode = Mode::from_str("u=g").unwrap();
        if let Mode::Clause(clauses) = mode {
            if let Action::Equal(bits) = clauses[0].actions[0] {
                assert_eq!(bits.copy_from, Who::Group);
            } else {
                panic!("Expected Action::Equal");
            }
        }
    }

    #[test]
    fn parse_mode_copy_from_other() {
        // ug=o means "set user and group permissions to whatever other has"
        let mode = Mode::from_str("ug=o").unwrap();
        if let Mode::Clause(clauses) = mode {
            assert_eq!(clauses[0].who, Who::User | Who::Group);
            if let Action::Equal(bits) = clauses[0].actions[0] {
                assert_eq!(bits.copy_from, Who::Other);
            } else {
                panic!("Expected Action::Equal");
            }
        }
    }

    #[test]
    fn parse_mode_copy_multiple_sources() {
        // o=ug means "set other permissions to user OR group"
        let mode = Mode::from_str("o=ug").unwrap();
        if let Mode::Clause(clauses) = mode {
            if let Action::Equal(bits) = clauses[0].actions[0] {
                assert_eq!(bits.copy_from, Who::User | Who::Group);
            } else {
                panic!("Expected Action::Equal");
            }
        }
    }

    #[test]
    fn parse_mode_copy_plus_literal() {
        // g=urx means "set group to user's permissions OR read and execute"
        let mode = Mode::from_str("g=urx").unwrap();
        if let Mode::Clause(clauses) = mode {
            if let Action::Equal(bits) = clauses[0].actions[0] {
                assert_eq!(bits.literal, 0o5); // r=4, x=1
                assert_eq!(bits.copy_from, Who::User);
            } else {
                panic!("Expected Action::Equal");
            }
        }
    }

    #[test]
    fn mode_apply_copy_user_to_group() {
        // g=u: copy user permissions to group
        // Starting: 0o750 (user=rwx, group=rx, other=0)
        // Result: group gets user's rwx -> 0o770
        assert_eq!(Mode::from_str("g=u").unwrap().apply_to(0o750), 0o770);

        // Starting: 0o640 (user=rw, group=r, other=0)
        // Result: group gets user's rw -> 0o660
        assert_eq!(Mode::from_str("g=u").unwrap().apply_to(0o640), 0o660);
    }

    #[test]
    fn mode_apply_copy_group_to_user() {
        // u=g: copy group permissions to user
        // Starting: 0o750 (user=rwx, group=rx, other=0)
        // Result: user gets group's rx -> 0o550
        assert_eq!(Mode::from_str("u=g").unwrap().apply_to(0o750), 0o550);
    }

    #[test]
    fn mode_apply_copy_user_to_group_and_other() {
        // go=u: copy user permissions to group and other
        // Starting: 0o700 (user=rwx, group=0, other=0)
        // Result: group and other get user's rwx -> 0o777
        assert_eq!(Mode::from_str("go=u").unwrap().apply_to(0o700), 0o777);
    }

    #[test]
    fn mode_apply_copy_other_to_user() {
        // u=o: copy other permissions to user
        // Starting: 0o705 (user=rwx, group=0, other=rx)
        // Result: user gets other's rx -> 0o505
        assert_eq!(Mode::from_str("u=o").unwrap().apply_to(0o705), 0o505);
    }

    #[test]
    fn mode_apply_copy_plus_literal() {
        // g=urx: copy user permissions to group, OR read and execute.
        assert_eq!(Mode::from_str("g=urx").unwrap().apply_to(0o100), 0o150);
        assert_eq!(Mode::from_str("g=urx").unwrap().apply_to(0o640), 0o670);
    }

    #[test]
    fn mode_apply_copy_with_literal() {
        // g=ux: copy user permissions to group, plus execute
        // Starting: 0o600 (user=rw, group=0, other=0)
        // Result: group gets user's rw (=6) OR x (=1) -> 0o670
        assert_eq!(Mode::from_str("g=ux").unwrap().apply_to(0o600), 0o670);
    }

    #[test]
    fn mode_apply_copy_other_to_user_and_group() {
        // ug=o: copy other permissions to user and group.
        assert_eq!(Mode::from_str("ug=o").unwrap().apply_to(0o705), 0o555);
        assert_eq!(Mode::from_str("ug=o").unwrap().apply_to(0o642), 0o222);
    }

    #[test]
    fn mode_apply_copy_from_multiple_sources() {
        // o=ug: copy user OR group permissions to other
        // Starting: 0o640 (user=rw, group=r, other=0)
        // Result: other gets (user rw | group r) = rw -> 0o646
        assert_eq!(Mode::from_str("o=ug").unwrap().apply_to(0o640), 0o646);
    }

    #[test]
    fn mode_apply_copy_order_dependence() {
        // u=g,g=u is evaluated left-to-right; it does not swap user/group permissions.
        let mode = 0o750;
        assert_eq!(Mode::from_str("u=g,g=u").unwrap().apply_to(mode), 0o550);
        assert_eq!(Mode::from_str("g=u,u=g").unwrap().apply_to(mode), 0o770);
    }

    #[test]
    fn mode_apply_copy_multiple_actions_in_single_clause() {
        // g=u-w: set group permissions to user's permissions, then remove write.
        assert_eq!(Mode::from_str("g=u-w").unwrap().apply_to(0o600), 0o640);
        assert_eq!(Mode::from_str("g=u-w").unwrap().apply_to(0o250), 0o200);

        // g=u+o: set group permissions to user's permissions, then add other's permissions.
        assert_eq!(Mode::from_str("g=u+o").unwrap().apply_to(0o641), 0o671);
        assert_eq!(Mode::from_str("g=u+o").unwrap().apply_to(0o502), 0o572);
    }

    #[test]
    fn mode_apply_copy_add_from_user() {
        // g+u: add user permissions to group
        // Starting: 0o741 (user=rwx, group=r, other=x)
        // Result: group gets r | rwx -> 0o771
        assert_eq!(Mode::from_str("g+u").unwrap().apply_to(0o741), 0o771);
    }

    #[test]
    fn mode_apply_copy_remove_from_user() {
        // g-u: remove user permissions from group
        // Starting: 0o777 (all have rwx)
        // User has rwx (7), so remove rwx from group -> 0o707
        assert_eq!(Mode::from_str("g-u").unwrap().apply_to(0o777), 0o707);
    }

    #[test]
    fn perm_bits_resolve_copies_correct_bits() {
        // Test PermBits::resolve directly
        let mode = 0o754u16; // user=rwx, group=rx, other=r

        // Copy from user (should get 7)
        assert_eq!(PermBits::copy_from(Who::User).resolve(mode), 0o7);

        // Copy from group (should get 5)
        assert_eq!(PermBits::copy_from(Who::Group).resolve(mode), 0o5);

        // Copy from other (should get 4)
        assert_eq!(PermBits::copy_from(Who::Other).resolve(mode), 0o4);

        // Copy from user | group (should get 7 | 5 = 7)
        assert_eq!(
            PermBits::copy_from(Who::User | Who::Group).resolve(mode),
            0o7
        );

        // Literal plus copy
        let bits = PermBits {
            literal: 0o1, // x
            copy_from: Who::Other,
        };
        assert_eq!(bits.resolve(mode), 0o5); // 4 | 1 = 5
    }
}
