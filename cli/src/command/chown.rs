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
        help = "Overwrite the output file (rewrite in place when --output is omitted)"
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
fn transform_entry<T>(entry: NormalEntry<T>, owner: &Ownership) -> NormalEntry<T> {
    let metadata = entry.metadata().clone();
    let own = crate::ext::ResolvedOwnership::from_metadata(&metadata);
    let (uid, uname, user_sid): (Option<u64>, String, Option<String>) = match owner.user.as_ref() {
        Some(OwnerSpecifier::Name(uname)) => (Some(u64::MAX), uname.clone(), None),
        Some(OwnerSpecifier::ID(uid)) => (Some(*uid), String::new(), None),
        Some(OwnerSpecifier::System(user)) => (
            Some(user.uid().unwrap_or(u64::MAX)),
            user.name().unwrap_or_default().into(),
            user.sid(),
        ),
        None => (
            own.uid,
            own.uname.clone().unwrap_or_default(),
            own.user_sid.clone(),
        ),
    };
    let (gid, gname, group_sid): (Option<u64>, String, Option<String>) = match owner.group.as_ref()
    {
        Some(OwnerSpecifier::Name(gname)) => (Some(u64::MAX), gname.clone(), None),
        Some(OwnerSpecifier::ID(gid)) => (Some(*gid), String::new(), None),
        Some(OwnerSpecifier::System(group)) => (
            Some(group.gid().unwrap_or(u64::MAX)),
            group.name().unwrap_or_default().into(),
            group.sid(),
        ),
        None => (
            own.gid,
            own.gname.clone().unwrap_or_default(),
            own.group_sid.clone(),
        ),
    };
    let metadata = metadata
        .with_owner_uid(uid.map(pna::OwnerUid::from))
        .with_owner_gid(gid.map(pna::OwnerGid::from))
        .with_owner_user_name(crate::command::core::permission::owner_name_opt(&uname))
        .with_owner_group_name(crate::command::core::permission::owner_group_name_opt(
            &gname,
        ))
        .with_permission_mode(own.mode.map(pna::PermissionMode::from))
        .with_owner_user_sid(
            user_sid.map(|s| pna::OwnerUserSid::new(s).expect("sid within owner-facet bound")),
        )
        .with_owner_group_sid(
            group_sid.map(|s| pna::OwnerGroupSid::new(s).expect("sid within owner-facet bound")),
        );
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
                Some(OwnerSpecifier::System(u)) => Some(OwnerSpecifier::System(u.primary_group()?)),
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

    #[cfg(unix)]
    #[test]
    fn lookup_user_login_group() {
        let ownership = RawOwnership::from_str("root:")
            .unwrap()
            .lookup_platform_owner(false, true)
            .unwrap();
        match ownership.group {
            Some(OwnerSpecifier::System(group)) => {
                assert_eq!(group.gid(), Some(0));
            }
            _ => panic!("login group must resolve to a system group"),
        }
    }

    #[test]
    fn transform_entry_replaces_sid_of_changed_side_only() {
        let metadata = pna::Metadata::new()
            .with_owner_uid(Some(pna::OwnerUid::from(1000)))
            .with_owner_gid(Some(pna::OwnerGid::from(1000)))
            .with_owner_user_name(Some(pna::OwnerUserName::new("user").unwrap()))
            .with_owner_group_name(Some(pna::OwnerGroupName::new("group").unwrap()))
            .with_owner_user_sid(Some(pna::OwnerUserSid::new("S-1-5-21-1111").unwrap()))
            .with_owner_group_sid(Some(pna::OwnerGroupSid::new("S-1-5-21-2222").unwrap()))
            .with_permission_mode(Some(pna::PermissionMode::from(0o644)));

        let mut user_builder = pna::FileEntryBuilder::new("target.txt".into()).unwrap();
        user_builder.metadata(metadata.clone());
        let user_changed = transform_entry(
            user_builder.build().unwrap(),
            &Ownership {
                user: Some(OwnerSpecifier::Name("new_user".into())),
                group: None,
            },
        );
        let m = user_changed.metadata();
        assert_eq!(m.owner_user_name().unwrap().as_str(), "new_user");
        assert_eq!(m.owner_uid().unwrap().get(), u64::MAX);
        assert!(m.owner_user_sid().is_none());
        assert_eq!(m.owner_group_name().unwrap().as_str(), "group");
        assert_eq!(m.owner_gid().unwrap().get(), 1000);
        assert_eq!(m.owner_group_sid().unwrap().as_str(), "S-1-5-21-2222");

        let mut group_builder = pna::FileEntryBuilder::new("target.txt".into()).unwrap();
        group_builder.metadata(metadata);
        let group_changed = transform_entry(
            group_builder.build().unwrap(),
            &Ownership {
                user: None,
                group: Some(OwnerSpecifier::Name("new_group".into())),
            },
        );
        let m = group_changed.metadata();
        assert_eq!(m.owner_group_name().unwrap().as_str(), "new_group");
        assert_eq!(m.owner_gid().unwrap().get(), u64::MAX);
        assert!(m.owner_group_sid().is_none());
        assert_eq!(m.owner_user_name().unwrap().as_str(), "user");
        assert_eq!(m.owner_uid().unwrap().get(), 1000);
        assert_eq!(m.owner_user_sid().unwrap().as_str(), "S-1-5-21-1111");
    }
}
