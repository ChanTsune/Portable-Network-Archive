use crate::{
    chunk::{Ace, AcePlatform, Flag, Identifier, OwnerType, Permission},
    cli::{PasswordArgs, SolidEntriesTransformStrategy, SolidEntriesTransformStrategyArgs},
    command::{
        ask_password,
        commons::{
            collect_split_archives, run_entries, run_transform_entry, TransformStrategyKeepSolid,
            TransformStrategyUnSolid,
        },
        Command,
    },
    ext::NormalEntryExt,
    utils::{GlobPatterns, PathPartExt},
};
use clap::{Parser, ValueHint};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::char,
    combinator::{map, opt},
    Parser as _,
};
use pna::{Chunk, NormalEntry, RawChunk};
use regex::Regex;
use std::{collections::HashSet, io, path::PathBuf, str::FromStr};

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub(crate) struct AclCommand {
    #[command(subcommand)]
    command: XattrCommands,
}

impl Command for AclCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        match self.command {
            XattrCommands::Get(cmd) => cmd.execute(),
            XattrCommands::Set(cmd) => cmd.execute(),
        }
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum XattrCommands {
    #[command(about = "Get acl of entries")]
    Get(GetAclCommand),
    #[command(about = "Set acl of entries")]
    Set(SetAclCommand),
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct GetAclCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for GetAclCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        archive_get_acl(self)
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct SetAclCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[arg(
        short = 'm',
        long,
        help = "Modify the ACL on the specified file. New entries will be added, and existing entries will be modified according to the entries argument."
    )]
    modify: Option<AclEntries>,
    #[arg(
        short = 'x',
        long,
        help = "Remove the ACL entries specified there from the access or default ACL of the specified files."
    )]
    remove: Option<AclEntries>,
    #[command(flatten)]
    transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for SetAclCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        archive_set_acl(self)
    }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct AclEntries {
    default: bool,
    owner: OwnerType,
    permissions: Option<Vec<String>>,
}

impl AclEntries {
    fn is_match(&self, ace: &Ace) -> bool {
        if self.default != ace.flags.contains(Flag::DEFAULT) {
            return false;
        }
        if self.owner != ace.owner_type {
            return false;
        }
        true
    }

    fn to_ace(&self) -> Ace {
        Ace {
            flags: if self.default {
                Flag::DEFAULT
            } else {
                Flag::empty()
            },
            owner_type: self.owner.clone(),
            allow: true,
            permission: if let Some(permissions) = &self.permissions {
                let permissions: HashSet<_> =
                    HashSet::from_iter(permissions.iter().map(|it| it.as_str()));
                let mut permission = Permission::empty();
                for (f, names) in Permission::PERMISSION_NAME_MAP {
                    if names.iter().any(|it| permissions.contains(it)) {
                        permission.insert(*f);
                    }
                }
                permission
            } else {
                Permission::empty()
            },
        }
    }
}

impl FromStr for AclEntries {
    type Err = String;

    /// `"[d[efault]:] [u[ser]:]uid [:perms]"`
    /// `"[d[efault]:] g[roup]:gid [:perms]"`
    /// `"[d[efault]:] m[ask][:] [:perms]"`
    /// `"[d[efault]:] o[ther][:] [:perms]"`
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn kw_default(s: &str) -> nom::IResult<&str, (char, Option<&str>)> {
            (char('d'), opt(tag("efault"))).parse(s)
        }
        fn kw_user(s: &str) -> nom::IResult<&str, (char, Option<&str>)> {
            (char('u'), opt(tag("ser"))).parse(s)
        }
        fn kw_group(s: &str) -> nom::IResult<&str, (char, Option<&str>)> {
            (char('g'), opt(tag("roup"))).parse(s)
        }
        fn kw_other(s: &str) -> nom::IResult<&str, (char, Option<&str>)> {
            (char('o'), opt(tag("ther"))).parse(s)
        }
        fn kw_mask(s: &str) -> nom::IResult<&str, (char, Option<&str>)> {
            (char('m'), opt(tag("ask"))).parse(s)
        }
        let rwx_regex =
            Regex::from_str("^([\\-r]?)([\\-w]?)([\\-x]?)$").expect("invalid 'rwx' regex");
        let (p, v) = map(
            (
                opt(map((kw_default, char(':')), |_| true)),
                alt((
                    map((kw_other, opt(char(':'))), |_| OwnerType::Other),
                    map((kw_mask, opt(char(':'))), |_| OwnerType::Mask),
                    map(
                        (kw_group, char(':'), take_while(|c| c != ':')),
                        |(_, _, gid)| {
                            if gid.is_empty() {
                                OwnerType::OwnerGroup
                            } else {
                                OwnerType::Group(Identifier(gid.into()))
                            }
                        },
                    ),
                    map(
                        (opt((kw_user, char(':'))), take_while(|c| c != ':')),
                        |(_, uid)| {
                            if uid.is_empty() {
                                OwnerType::Owner
                            } else {
                                OwnerType::User(Identifier(uid.into()))
                            }
                        },
                    ),
                )),
                opt(map(
                    (char(':'), take_while(|_| true)),
                    |(_, c): (_, &str)| {
                        if c.is_empty() {
                            Vec::new()
                        } else {
                            c.split(',')
                                .flat_map(|it| {
                                    if let Some(cap) = rwx_regex.captures(it) {
                                        cap.iter()
                                            .skip(1)
                                            .flatten()
                                            .map(|it| it.as_str().to_string())
                                            .filter(|it| *it != "-")
                                            .collect()
                                    } else {
                                        vec![it.to_string()]
                                    }
                                })
                                .collect()
                        }
                    },
                )),
            ),
            |(d, owner, permissions)| AclEntries {
                default: d.unwrap_or_default(),
                owner,
                permissions,
            },
        )
        .parse_complete(s)
        .map_err(|it| it.to_string())?;
        if !p.is_empty() {
            return Err(format!("unexpected value: {}", p));
        }
        Ok(v)
    }
}

fn archive_get_acl(args: GetAclCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
    if args.files.is_empty() {
        return Ok(());
    }
    let globs = GlobPatterns::new(args.files)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let archives = collect_split_archives(&args.archive)?;

    run_entries(
        archives,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            let name = entry.header().path();
            let permission = entry.metadata().permission();
            if globs.matches_any(name) {
                println!("# file: {}", name);
                println!("# owner: {}", permission.map_or("-", |it| it.uname()));
                println!("# group: {}", permission.map_or("-", |it| it.gname()));
                for (platform, acl) in entry.acl()? {
                    println!("# platform: {}", platform);
                    for ace in acl {
                        println!("{}", ace);
                    }
                }
            }
            Ok(())
        },
    )?;
    Ok(())
}

fn archive_set_acl(args: SetAclCommand) -> io::Result<()> {
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
                    Ok(Some(transform_entry(
                        entry,
                        args.modify.as_ref(),
                        args.remove.as_ref(),
                    )))
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
                    Ok(Some(transform_entry(
                        entry,
                        args.modify.as_ref(),
                        args.remove.as_ref(),
                    )))
                } else {
                    Ok(Some(entry))
                }
            },
            TransformStrategyKeepSolid,
        ),
    }
}

#[inline]
fn transform_entry<T>(
    entry: NormalEntry<T>,
    modify: Option<&AclEntries>,
    remove: Option<&AclEntries>,
) -> NormalEntry<T>
where
    T: Clone,
    RawChunk<T>: Chunk,
    RawChunk<T>: From<RawChunk>,
{
    let platform = AcePlatform::General;
    let platform = &platform;
    let mut acls = entry.acl().unwrap_or_default();
    let acl = if let Some(acl) = acls.get_mut(platform) {
        acl
    } else {
        return entry;
    };

    let extra_without_known = entry
        .extra_chunks()
        .iter()
        .filter(|it| it.ty() != crate::chunk::faCe && it.ty() != crate::chunk::faCl)
        .cloned();
    if let Some(modify) = modify {
        let ace = modify.to_ace();
        let item = acl.iter_mut().find(|it| modify.is_match(it));
        if let Some(item) = item {
            log::debug!("Modifying ace {} to {}", item, ace);
            item.permission = ace.permission;
        } else {
            log::debug!("Adding ace {} ", ace);
            acl.push(ace);
        }
    }
    if let Some(remove) = remove {
        log::debug!("Removing ace {}", remove.to_ace());
        acl.retain(|it| !remove.is_match(it));
    }
    let mut acl_chunks = Vec::new();
    for (platform, aces) in acls {
        acl_chunks.push(RawChunk::from_data(crate::chunk::faCl, platform.to_bytes()).into());
        for ace in aces {
            acl_chunks.push(RawChunk::from_data(crate::chunk::faCe, ace.to_bytes()).into());
        }
    }
    let extra_chunks = acl_chunks
        .into_iter()
        .chain(extra_without_known)
        .collect::<Vec<_>>();
    entry.with_extra_chunks(&extra_chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_acl_user() {
        assert_eq!(
            AclEntries::from_str("uname").unwrap(),
            AclEntries {
                default: false,
                owner: OwnerType::User(Identifier("uname".into())),
                permissions: None,
            }
        );
        assert_eq!(
            AclEntries::from_str("user:").unwrap(),
            AclEntries {
                default: false,
                owner: OwnerType::Owner,
                permissions: None,
            }
        );
        assert_eq!(
            AclEntries::from_str("uname:").unwrap(),
            AclEntries {
                default: false,
                owner: OwnerType::User(Identifier("uname".into())),
                permissions: Some(Vec::new()),
            }
        );
    }

    #[test]
    fn parse_acl_group() {
        assert_eq!(
            AclEntries::from_str("g:").unwrap(),
            AclEntries {
                default: false,
                owner: OwnerType::OwnerGroup,
                permissions: None,
            }
        );
        assert_eq!(
            AclEntries::from_str("group:gname").unwrap(),
            AclEntries {
                default: false,
                owner: OwnerType::Group(Identifier("gname".into())),
                permissions: None,
            }
        );
        assert_eq!(
            AclEntries::from_str("g::").unwrap(),
            AclEntries {
                default: false,
                owner: OwnerType::OwnerGroup,
                permissions: Some(Vec::new()),
            }
        );
        assert_eq!(
            AclEntries::from_str("group:gname:").unwrap(),
            AclEntries {
                default: false,
                owner: OwnerType::Group(Identifier("gname".into())),
                permissions: Some(Vec::new()),
            }
        );
    }

    #[test]
    fn parse_acl_mask() {
        assert_eq!(
            AclEntries::from_str("m:").unwrap(),
            AclEntries {
                default: false,
                owner: OwnerType::Mask,
                permissions: None,
            }
        );
        assert_eq!(
            AclEntries::from_str("mask:").unwrap(),
            AclEntries {
                default: false,
                owner: OwnerType::Mask,
                permissions: None,
            }
        );
    }

    #[test]
    fn parse_acl_other() {
        assert_eq!(
            AclEntries::from_str("o:").unwrap(),
            AclEntries {
                default: false,
                owner: OwnerType::Other,
                permissions: None,
            }
        );
        assert_eq!(
            AclEntries::from_str("other:").unwrap(),
            AclEntries {
                default: false,
                owner: OwnerType::Other,
                permissions: None,
            }
        );
    }

    #[test]
    fn parse_acl_rwx() {
        assert_eq!(
            AclEntries::from_str("d:u::rwx").unwrap(),
            AclEntries {
                default: true,
                owner: OwnerType::Owner,
                permissions: Some(vec!["r".into(), "w".into(), "x".into()]),
            }
        );
        assert_eq!(
            AclEntries::from_str("d:u::rw-").unwrap(),
            AclEntries {
                default: true,
                owner: OwnerType::Owner,
                permissions: Some(vec!["r".into(), "w".into()]),
            }
        );
        assert_eq!(
            AclEntries::from_str("d:u::r-x").unwrap(),
            AclEntries {
                default: true,
                owner: OwnerType::Owner,
                permissions: Some(vec!["r".into(), "x".into()]),
            }
        );
        assert_eq!(
            AclEntries::from_str("d:u::-w-").unwrap(),
            AclEntries {
                default: true,
                owner: OwnerType::Owner,
                permissions: Some(vec!["w".into()]),
            }
        );
        assert_eq!(
            AclEntries::from_str("d:u::---").unwrap(),
            AclEntries {
                default: true,
                owner: OwnerType::Owner,
                permissions: Some(vec![]),
            }
        );
    }
}
