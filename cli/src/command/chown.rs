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
    utils::{
        fs::{Group, User},
        GlobPatterns, PathPartExt,
    },
};
use clap::{ArgGroup, Parser, ValueHint};
use pna::NormalEntry;
use std::{io, ops::Not, path::PathBuf, str::FromStr};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(
    group(ArgGroup::new("lookup").args(["owner_lookup", "no_owner_lookup"])),
)]
pub(crate) struct ChownCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(help = "owner[:group]|:group")]
    owner: RawOwnership,
    #[arg(long, help = "force numeric owner and group IDs (no name resolution)")]
    numeric_owner: bool,
    #[arg(long, help = "resolve user and group (default)")]
    owner_lookup: bool,
    #[arg(long, help = "do not resolve user and group")]
    no_owner_lookup: bool,
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[command(flatten)]
    transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for ChownCommand {
    #[inline]
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

    let owner = args
        .owner
        .lookup_platform_owner(args.numeric_owner, !args.no_owner_lookup)?;

    let archives = collect_split_archives(&args.archive)?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => run_transform_entry(
            args.archive.remove_part().unwrap(),
            archives,
            || password.as_deref(),
            |entry| {
                let entry = entry?;
                if globs.matches_any(entry.header().path()) {
                    Ok(Some(transform_entry(entry, &owner)))
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
                    Ok(Some(transform_entry(entry, &owner)))
                } else {
                    Ok(Some(entry))
                }
            },
            TransformStrategyKeepSolid,
        ),
    }
}

#[inline]
fn transform_entry<T>(entry: NormalEntry<T>, owner: &Ownership) -> NormalEntry<T> {
    let metadata = entry.metadata().clone();
    let permission = metadata.permission().map(|p| {
        let user = owner.user.as_ref().map(|it| match it {
            OwnerSpecifier::Name(uname) => (u64::MAX, uname.into()),
            OwnerSpecifier::ID(uid) => (*uid, String::new()),
            OwnerSpecifier::System(user) => (
                user.uid().unwrap_or(u64::MAX),
                user.name().unwrap_or_default().into(),
            ),
        });
        let (uid, uname) = user.unwrap_or_else(|| (p.uid(), p.uname().into()));

        let group = owner.group.as_ref().map(|it| match it {
            OwnerSpecifier::Name(gname) => (u64::MAX, gname.into()),
            OwnerSpecifier::ID(gid) => (*gid, String::new()),
            OwnerSpecifier::System(group) => (
                group.gid().unwrap_or(u64::MAX),
                group.name().unwrap_or_default().into(),
            ),
        });
        let (gid, gname) = group.unwrap_or_else(|| (p.gid(), p.gname().into()));
        pna::Permission::new(uid, uname, gid, gname, p.permissions())
    });
    entry.with_metadata(metadata.with_permission(permission))
}

pub(crate) enum OwnerSpecifier<T> {
    Name(String),
    ID(u64),
    System(T),
}

pub(crate) struct Ownership {
    user: Option<OwnerSpecifier<User>>,
    group: Option<OwnerSpecifier<Group>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct RawOwnership {
    user: Option<String>,
    group: Option<String>,
}

impl RawOwnership {
    #[inline]
    fn lookup_platform_owner(self, numeric_owner: bool, lookup: bool) -> io::Result<Ownership> {
        let user = match self.user {
            Some(user) if lookup => Some(OwnerSpecifier::System(if numeric_owner {
                User::from_uid(
                    user.parse()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
                )?
            } else {
                User::from_name(&user)?
            })),
            Some(user) => Some(if numeric_owner {
                OwnerSpecifier::ID(
                    user.parse()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
                )
            } else {
                OwnerSpecifier::Name(user)
            }),
            None => None,
        };
        let group = match self.group {
            Some(group) if lookup => Some(OwnerSpecifier::System(if numeric_owner {
                Group::from_gid(
                    group
                        .parse()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
                )?
            } else {
                Group::from_name(&group)?
            })),
            Some(group) => Some(if numeric_owner {
                OwnerSpecifier::ID(
                    group
                        .parse()
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?,
                )
            } else {
                OwnerSpecifier::Name(group)
            }),
            None => None,
        };
        Ok(Ownership { user, group })
    }
}

impl FromStr for RawOwnership {
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
            RawOwnership::from_str("user").unwrap(),
            RawOwnership {
                user: Some("user".into()),
                group: None,
            }
        );
    }

    #[test]
    fn group() {
        assert_eq!(
            RawOwnership::from_str(":group").unwrap(),
            RawOwnership {
                user: None,
                group: Some("group".into()),
            }
        );
    }

    #[test]
    fn user_group() {
        assert_eq!(
            RawOwnership::from_str("user:group").unwrap(),
            RawOwnership {
                user: Some("user".into()),
                group: Some("group".into()),
            }
        );
    }
}
