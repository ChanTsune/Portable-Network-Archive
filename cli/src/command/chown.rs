#[cfg(any(unix, windows))]
use crate::utils::fs::{Group, User};
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
use pna::RegularEntry;
use std::ops::Not;
use std::{io, path::PathBuf, str::FromStr};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ChownCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(help = "owner[:group]|:group")]
    owner: Owner,
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[command(flatten)]
    transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for ChownCommand {
    fn execute(self) -> io::Result<()> {
        archive_chown(self)
    }
}

fn archive_chown(args: ChownCommand) -> io::Result<()> {
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
                    Ok(Some(transform_entry(entry, &args.owner)))
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
                    Ok(Some(transform_entry(entry, &args.owner)))
                } else {
                    Ok(Some(entry))
                }
            },
            TransformStrategyKeepSolid,
        ),
    }
}

#[inline]
fn transform_entry<T>(entry: RegularEntry<T>, owner: &Owner) -> RegularEntry<T> {
    let metadata = entry.metadata().clone();
    let permission = metadata.permission().map(|p| {
        let user = owner.user();
        #[cfg(unix)]
        let user = user
            .and_then(|it| User::from_name(it).map(|it| (it.as_raw().into(), it.name().into())));
        #[cfg(windows)]
        let user = user.and_then(|it| User::from_name(it).map(|it| (u64::MAX, it.name().into())));
        #[cfg(not(any(unix, windows)))]
        let user = user.map(|_| (p.uid(), p.uname().into()));
        let (uid, uname) = user.unwrap_or_else(|| (p.uid(), p.uname().into()));

        let group = owner.group();
        #[cfg(unix)]
        let group = group
            .and_then(|it| Group::from_name(it).map(|it| (it.as_raw().into(), it.name().into())));
        #[cfg(windows)]
        let group =
            group.and_then(|it| Group::from_name(it).map(|it| (u64::MAX, it.name().into())));
        #[cfg(not(any(unix, windows)))]
        let group = group.map(|_| (p.gid(), p.gname().into()));
        let (gid, gname) = group.unwrap_or_else(|| (p.gid(), p.gname().into()));
        pna::Permission::new(uid, uname, gid, gname, p.permissions())
    });
    entry.with_metadata(metadata.with_permission(permission))
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Owner {
    user: Option<String>,
    group: Option<String>,
}

impl Owner {
    #[inline]
    pub(crate) fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    #[inline]
    pub(crate) fn group(&self) -> Option<&str> {
        self.group.as_deref()
    }
}

impl FromStr for Owner {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s == ":" {
            return Err("owner must not be empty".into());
        }
        let (user, group) = if let Some((user, group)) = s.split_once(':') {
            (
                user.is_empty().not().then(|| user.into()),
                group.is_empty().not().then(|| group.into()),
            )
        } else {
            (Some(s.into()), None)
        };
        Ok(Self { user, group })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_from_str_user() {
        assert_eq!(
            Owner::from_str("user").unwrap(),
            Owner {
                user: Some("user".into()),
                group: None,
            }
        );
    }

    #[test]
    fn group() {
        assert_eq!(
            Owner::from_str(":group").unwrap(),
            Owner {
                user: None,
                group: Some("group".into()),
            }
        );
    }

    #[test]
    fn user_group() {
        assert_eq!(
            Owner::from_str("user:group").unwrap(),
            Owner {
                user: Some("user".into()),
                group: Some("group".into()),
            }
        );
    }
}
