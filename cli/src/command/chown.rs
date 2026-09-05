use crate::{
    cli::{ArchiveFileArgs, PasswordArgs, SolidEntriesTransformStrategyArgs},
    command::{
        Command, ask_password,
        core::{
            Umask, resolve_rewrite_output,
            rewrite::{EntryTransform, execute_archive_transform},
        },
    },
    utils::{
        GlobPatterns,
        fs::{Group, User},
    },
};
use clap::{ArgAction, Parser, ValueHint, builder::ArgPredicate};
use pna::NormalEntry;
use std::{borrow::Cow, io, ops::Not, path::PathBuf, str::FromStr};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ChownCommand {
    #[command(flatten)]
    archive: ArchiveFileArgs,
    #[arg(help = "owner[:group]|:group")]
    owner: RawOwnership,
    #[arg(long, help = "force numeric owner and group IDs (no name resolution)")]
    numeric_owner: bool,
    #[arg(
        long,
        help = "resolve user and group (default)",
        default_value_t = true,
        default_value_if("no_owner_lookup", ArgPredicate::Equals("true".into()), "false"),
        conflicts_with = "no_owner_lookup"
    )]
    owner_lookup: bool,
    #[arg(long, action = ArgAction::SetTrue, help = "do not resolve user and group")]
    no_owner_lookup: (),
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[arg(long, help = "Output file path", value_hint = ValueHint::FilePath)]
    output: Option<PathBuf>,
    #[arg(
        long,
        conflicts_with = "no_overwrite",
        requires = "output",
        help = "Overwrite output file"
    )]
    overwrite: bool,
    #[arg(
        long,
        action = ArgAction::SetTrue,
        help = "Do not overwrite output file. This is the inverse option of --overwrite",
        requires = "output"
    )]
    no_overwrite: (),
    #[command(flatten)]
    transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for ChownCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        archive_chown(self, ctx.umask())
    }
}

#[hooq::hooq(anyhow)]
fn archive_chown(args: ChownCommand, umask: Umask) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    if args.files.is_empty() {
        return Ok(());
    }
    let globs = GlobPatterns::new(args.files.iter().map(|p| p.as_ref()))?;

    let owner = args
        .owner
        .lookup_platform_owner(args.numeric_owner, args.owner_lookup)?;
    let destination = resolve_rewrite_output(&args.archive.file, args.output, args.overwrite)?;
    execute_archive_transform(
        &args.archive.file,
        destination,
        umask,
        password.as_deref(),
        args.transform_strategy.strategy(),
        ChownTransform { globs, owner },
    )
}

struct ChownTransform<'g> {
    globs: GlobPatterns<'g>,
    owner: Ownership,
}

impl EntryTransform for ChownTransform<'_> {
    fn transform<'d>(
        &mut self,
        entry: NormalEntry<Cow<'d, [u8]>>,
    ) -> io::Result<Option<NormalEntry<Cow<'d, [u8]>>>> {
        Ok(Some(if self.globs.matches_any(entry.name()) {
            transform_entry(entry, &self.owner)
        } else {
            entry
        }))
    }

    fn patterns(&self) -> Option<&GlobPatterns<'_>> {
        Some(&self.globs)
    }
}

#[inline]
#[allow(deprecated)]
fn transform_entry<T>(entry: NormalEntry<T>, owner: &Ownership) -> NormalEntry<T> {
    let metadata = entry.metadata().clone();
    let own = crate::ext::ResolvedOwnership::from_metadata(&metadata);
    let (uid, uname): (Option<u64>, String) = match owner.user.as_ref() {
        Some(OwnerSpecifier::Name(uname)) => (Some(u64::MAX), uname.clone()),
        Some(OwnerSpecifier::ID(uid)) => (Some(*uid), String::new()),
        Some(OwnerSpecifier::System(user)) => (
            Some(user.uid().unwrap_or(u64::MAX)),
            user.name().unwrap_or_default().into(),
        ),
        None => (own.uid, own.uname.clone().unwrap_or_default()),
    };
    let (gid, gname): (Option<u64>, String) = match owner.group.as_ref() {
        Some(OwnerSpecifier::Name(gname)) => (Some(u64::MAX), gname.clone()),
        Some(OwnerSpecifier::ID(gid)) => (Some(*gid), String::new()),
        Some(OwnerSpecifier::System(group)) => (
            Some(group.gid().unwrap_or(u64::MAX)),
            group.name().unwrap_or_default().into(),
        ),
        None => (own.gid, own.gname.clone().unwrap_or_default()),
    };
    let metadata =
        metadata
            .with_permission(None)
            .with_owner_uid(uid.map(pna::OwnerUid::from))
            .with_owner_gid(gid.map(pna::OwnerGid::from))
            .with_owner_user_name(crate::command::core::permission::owner_name_opt(&uname))
            .with_owner_group_name(crate::command::core::permission::owner_group_name_opt(
                &gname,
            ))
            .with_permission_mode(own.mode.map(pna::PermissionMode::from))
            .with_owner_user_sid(
                own.user_sid.clone().map(|s| {
                    pna::OwnerUserSid::new(s).expect("rescued sid within owner-facet bound")
                }),
            )
            .with_owner_group_sid(own.group_sid.clone().map(|s| {
                pna::OwnerGroupSid::new(s).expect("rescued sid within owner-facet bound")
            }));
    entry.with_metadata(metadata)
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
    use_login_group: bool,
}

impl RawOwnership {
    #[inline]
    fn lookup_platform_owner(self, numeric_owner: bool, lookup: bool) -> io::Result<Ownership> {
        if self.use_login_group && !lookup {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot use 'user:' format with --no-owner-lookup",
            ));
        }

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
            _ if self.use_login_group => match &user {
                Some(OwnerSpecifier::System(u)) => {
                    let gid = u.primary_gid().ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::Unsupported,
                            "cannot get user's primary group",
                        )
                    })?;
                    Some(OwnerSpecifier::System(Group::from_gid(gid)?))
                }
                _ => None,
            },
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
        let (user, group, use_login_group) = if let Some((user, group)) = s.split_once(':') {
            (
                user.is_empty().not().then(|| user.into()),
                group.is_empty().not().then(|| group.into()),
                !user.is_empty() && group.is_empty(),
            )
        } else {
            (Some(s.into()), None, false)
        };
        Ok(Self {
            user,
            group,
            use_login_group,
        })
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
                use_login_group: false,
            }
        );
    }

    #[test]
    fn owner_from_str_user_login_group() {
        assert_eq!(
            RawOwnership::from_str("user:").unwrap(),
            RawOwnership {
                user: Some("user".into()),
                group: None,
                use_login_group: true,
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
                use_login_group: false,
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
                use_login_group: false,
            }
        );
    }
}
