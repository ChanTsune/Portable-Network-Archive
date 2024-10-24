use crate::{
    cli::{PasswordArgs, SolidEntriesTransformStrategy, SolidEntriesTransformStrategyArgs},
    command::{
        ask_password,
        commons::{run_transform_entry, TransformStrategyKeepSolid, TransformStrategyUnSolid},
        Command,
    },
    utils::{GlobPatterns, PathPartExt},
};
use clap::{Parser, ValueHint};
use pna::NormalEntry;
use std::{
    io,
    path::PathBuf,
    str::{Chars, FromStr},
};

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
    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => run_transform_entry(
            args.archive.remove_part().unwrap(),
            &args.archive,
            || password.as_deref(),
            |entry| {
                let entry = entry?;
                if globs.matches_any(entry.header().path()) {
                    Ok(Some(transform_entry(entry, args.mode)))
                } else {
                    Ok(Some(entry))
                }
            },
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => run_transform_entry(
            args.archive.remove_part().unwrap(),
            &args.archive,
            || password.as_deref(),
            |entry| {
                let entry = entry?;
                if globs.matches_any(entry.header().path()) {
                    Ok(Some(transform_entry(entry, args.mode)))
                } else {
                    Ok(Some(entry))
                }
            },
            TransformStrategyKeepSolid,
        ),
    }
}

#[inline]
fn transform_entry<T>(entry: NormalEntry<T>, mode: Mode) -> NormalEntry<T> {
    let metadata = entry.metadata().clone();
    let permission = metadata.permission().map(|p| {
        let mode = mode.apply_to(p.permissions());
        pna::Permission::new(p.uid(), p.uname().into(), p.gid(), p.gname().into(), mode)
    });
    entry.with_metadata(metadata.with_permission(permission))
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Target {
    User,
    Group,
    Other,
    All,
}

impl Target {
    #[inline]
    const fn apply_to(&self, n: u16) -> u16 {
        match self {
            Target::User => n << 6,
            Target::Group => n << 3,
            Target::Other => n,
            Target::All => n << 6 | n << 3 | n,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Mode {
    Num(u16),
    Equal(Target, u8),
    Plus(Target, u8),
    Minus(Target, u8),
}

impl Mode {
    const OWNER_MASK: u16 = 0o077;
    const GROUP_MASK: u16 = 0o707;
    const OTHER_MASK: u16 = 0o770;
    #[inline]
    pub(crate) const fn apply_to(&self, mode: u16) -> u16 {
        match self {
            Mode::Num(mode) => *mode,
            Mode::Equal(t, m) => match t {
                Target::User => mode & Self::OWNER_MASK | t.apply_to(*m as u16),
                Target::Group => mode & Self::GROUP_MASK | t.apply_to(*m as u16),
                Target::Other => mode & Self::OTHER_MASK | t.apply_to(*m as u16),
                Target::All => t.apply_to(*m as u16),
            },
            Mode::Plus(t, m) => mode | t.apply_to(*m as u16),
            Mode::Minus(t, m) => mode & !t.apply_to(*m as u16),
        }
    }
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #[inline]
        fn parse_mode(chars: Chars) -> Result<u8, <Mode as FromStr>::Err> {
            let mut mode = 0;
            for c in chars {
                match c {
                    'x' => mode |= 1,
                    'w' => mode |= 2,
                    'r' => mode |= 4,
                    _ => {
                        return Err(format!(
                            "unexpected character '{}'. excepted one of 'r', 'w' or 'x'",
                            c
                        ))
                    }
                };
            }
            Ok(mode)
        }

        #[inline]
        fn parse_alphabetic_mode(
            mut chars: Chars,
            target: Target,
        ) -> Result<Mode, <Mode as FromStr>::Err> {
            match chars
                .next()
                .ok_or_else(|| "excepted one of '+', '-' or '='".to_string())?
            {
                '+' => Ok(Mode::Plus(target, parse_mode(chars)?)),
                '-' => Ok(Mode::Minus(target, parse_mode(chars)?)),
                '=' => Ok(Mode::Equal(target, parse_mode(chars)?)),
                m => Err(format!(
                    "unexpected character '{}'. excepted one of '+', '-' or '='",
                    m
                )),
            }
        }
        if s.is_empty() {
            return Err("mode must not be empty".into());
        }
        if s.chars().all(|c| c.is_ascii_digit()) {
            return if s.len() == 3 {
                u16::from_str_radix(s, 8)
                    .map(Self::Num)
                    .map_err(|e| e.to_string())
            } else {
                Err(format!("invalid mode length {}", s.len()))
            };
        }
        let mut chars = s.chars();
        match chars.next().expect("") {
            'u' => parse_alphabetic_mode(chars, Target::User),
            'g' => parse_alphabetic_mode(chars, Target::Group),
            'o' => parse_alphabetic_mode(chars, Target::Other),
            'a' => parse_alphabetic_mode(chars, Target::All),
            '+' | '-' | '=' => parse_alphabetic_mode(s.chars(), Target::All),
            first => Err(format!("unexpected character '{}'", first)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_digit_mode() {
        assert_eq!(Mode::from_str("755").unwrap(), Mode::Num(0o755));
        assert_eq!(Mode::from_str("000").unwrap(), Mode::Num(0o000));
    }

    #[test]
    fn parse_alphabetic_mode() {
        assert_eq!(
            Mode::from_str("=rwx").unwrap(),
            Mode::Equal(Target::All, 0o7),
        );
        assert_eq!(
            Mode::from_str("=rw").unwrap(),
            Mode::Equal(Target::All, 0o6),
        );
        assert_eq!(Mode::from_str("+x").unwrap(), Mode::Plus(Target::All, 0o1));
        assert_eq!(Mode::from_str("-w").unwrap(), Mode::Minus(Target::All, 0o2));
    }

    #[test]
    fn parse_alphabetic_mode_with_user() {
        assert_eq!(
            Mode::from_str("u=rwx").unwrap(),
            Mode::Equal(Target::User, 0o7),
        );
        assert_eq!(
            Mode::from_str("g=rw").unwrap(),
            Mode::Equal(Target::Group, 0o6),
        );
        assert_eq!(
            Mode::from_str("o+x").unwrap(),
            Mode::Plus(Target::Other, 0o1),
        );
        assert_eq!(
            Mode::from_str("a-w").unwrap(),
            Mode::Minus(Target::All, 0o2),
        );
    }

    #[test]
    fn mode_apply_to() {
        assert_eq!(Mode::from_str("755").unwrap().apply_to(0o764), 0o755);
        assert_eq!(Mode::from_str("+x").unwrap().apply_to(0o664), 0o775);
        assert_eq!(Mode::from_str("o+r").unwrap().apply_to(0o600), 0o604);
        assert_eq!(Mode::from_str("u-r").unwrap().apply_to(0o600), 0o200);
        assert_eq!(Mode::from_str("g=rw").unwrap().apply_to(0o777), 0o767);
    }
}
