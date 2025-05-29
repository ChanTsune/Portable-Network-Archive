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
    ext::{Acls, NormalEntryExt, PermissionExt},
    utils::{GlobPatterns, PathPartExt},
};
use clap::{ArgGroup, Parser, ValueHint};
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::char,
    combinator::{map, opt},
    Parser as _,
};
use pna::{Chunk, NormalEntry, RawChunk};
use regex::Regex;
use std::{
    collections::{HashMap, HashSet},
    fs, io,
    path::PathBuf,
    str::FromStr,
};

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
#[allow(clippy::large_enum_variant)]
pub(crate) enum XattrCommands {
    #[command(about = "Get acl of entries")]
    Get(GetAclCommand),
    #[command(about = "Set acl of entries")]
    Set(SetAclCommand),
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct GetAclCommand {
    #[arg(long, help = "Display specified ACL platform", value_delimiter = ',')]
    platform: Vec<AcePlatform>,
    #[arg(short, long, help = "List numeric user and group IDs")]
    numeric: bool,
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
#[command(
    group(ArgGroup::new("set-flags").args(["set", "modify"])),
)]
pub(crate) struct SetAclCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[arg(long, help = "Set the ACL on the specified file.")]
    set: Option<AclEntries>,
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
    #[arg(
        long,
        help = "Target ACL platform",
        default_value_t = AcePlatform::General
    )]
    platform: AcePlatform,
    #[arg(
        long,
        help = "Restore a permission backup created by `pna acl get *` or similar. All permissions of a complete directory subtree are restored using this mechanism. If a dash (-) is given as the file name, reads from standard input",
        value_hint = ValueHint::FilePath
    )]
    restore: Option<String>,
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
    allow: Option<bool>,
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
        if let Some(allow) = self.allow {
            if allow != ace.allow {
                return false;
            }
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
            allow: self.allow.unwrap_or(true),
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

    /// `"[d[efault]:] [u[ser]:]uid [:(allow|deny)] [:perms]"`
    /// `"[d[efault]:] g[roup]:gid [:(allow|deny)] [:perms]"`
    /// `"[d[efault]:] m[ask][:] [:(allow|deny)] [:perms]"`
    /// `"[d[efault]:] o[ther][:] [:(allow|deny)] [:perms]"`
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn kw_default(s: &str) -> nom::IResult<&str, (char, Option<&str>)> {
            (char('d'), opt(tag("efault"))).parse(s)
        }
        fn kw_allow(s: &str) -> nom::IResult<&str, &str> {
            tag("allow").parse(s)
        }
        fn kw_deny(s: &str) -> nom::IResult<&str, &str> {
            tag("deny").parse(s)
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
                map(
                    opt((
                        char(':'),
                        alt((map(kw_allow, |_| true), map(kw_deny, |_| false))),
                    )),
                    |a| a.map(|(_, a)| a),
                ),
                opt(map(
                    (char(':'), take_while(|_| true)),
                    |(_, c): (_, &str)| {
                        if c.is_empty() {
                            Vec::new()
                        } else {
                            let separator = if c.contains(',') { ',' } else { '|' };
                            c.split(separator)
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
            |(d, owner, allow, permissions)| AclEntries {
                default: d.unwrap_or_default(),
                allow,
                owner,
                permissions,
            },
        )
        .parse_complete(s)
        .map_err(|it| it.to_string())?;
        if !p.is_empty() {
            return Err(format!("unexpected value: {p}"));
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
    let platforms = args.platform.into_iter().collect::<HashSet<_>>();
    let numeric_owner = args.numeric;

    let archives = collect_split_archives(&args.archive)?;

    run_entries(
        archives,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            let name = entry.header().path();
            if globs.matches_any(name) {
                let permission = entry.metadata().permission();
                println!("# file: {name}");
                if let Some(permission) = permission {
                    println!("# owner: {}", permission.owner_display(numeric_owner));
                    println!("# group: {}", permission.group_display(numeric_owner));
                } else {
                    println!("# owner: ");
                    println!("# group: ");
                }
                for (platform, acl) in entry
                    .acl()?
                    .into_iter()
                    .filter(|(p, _)| platforms.is_empty() || platforms.contains(p))
                {
                    println!("# platform: {platform}");
                    for ace in acl {
                        println!("{ace}");
                    }
                }
                println!();
            }
            Ok(())
        },
    )?;
    Ok(())
}

fn archive_set_acl(args: SetAclCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
    let set_strategy = if let Some("-") = args.restore.as_deref() {
        SetAclsStrategy::Restore(parse_acl_dump(io::stdin().lock())?)
    } else if let Some(path) = args.restore.as_deref() {
        SetAclsStrategy::Restore(parse_acl_dump(io::BufReader::new(fs::File::open(path)?))?)
    } else if args.files.is_empty() {
        return Ok(());
    } else {
        let globs = GlobPatterns::new(args.files)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        SetAclsStrategy::Apply {
            globs,
            set: args.set,
            modify: args.modify,
            remove: args.remove,
            platform: args.platform,
        }
    };

    let archives = collect_split_archives(&args.archive)?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => run_transform_entry(
            args.archive.remove_part().unwrap(),
            archives,
            || password.as_deref(),
            |entry| Ok(Some(set_strategy.transform_entry(entry?))),
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => run_transform_entry(
            args.archive.remove_part().unwrap(),
            archives,
            || password.as_deref(),
            |entry| Ok(Some(set_strategy.transform_entry(entry?))),
            TransformStrategyKeepSolid,
        ),
    }
}

enum SetAclsStrategy {
    Restore(HashMap<String, Acls>),
    Apply {
        globs: GlobPatterns,
        set: Option<AclEntries>,
        modify: Option<AclEntries>,
        remove: Option<AclEntries>,
        platform: AcePlatform,
    },
}

impl SetAclsStrategy {
    #[inline]
    fn transform_entry<T>(&self, entry: NormalEntry<T>) -> NormalEntry<T>
    where
        T: Clone,
        RawChunk<T>: Chunk,
        RawChunk<T>: From<RawChunk>,
    {
        match self {
            Self::Restore(restore) => {
                if let Some(acls) = restore.get(entry.header().path().as_str()) {
                    let extra_without_known = entry
                        .extra_chunks()
                        .iter()
                        .filter(|it| it.ty() != crate::chunk::faCe && it.ty() != crate::chunk::faCl)
                        .cloned();
                    let mut acl_chunks = Vec::new();
                    for (platform, aces) in acls {
                        acl_chunks.push(
                            RawChunk::from_data(crate::chunk::faCl, platform.to_bytes()).into(),
                        );
                        for ace in aces {
                            acl_chunks.push(
                                RawChunk::from_data(crate::chunk::faCe, ace.to_bytes()).into(),
                            );
                        }
                    }
                    let extra_chunks = acl_chunks
                        .into_iter()
                        .chain(extra_without_known)
                        .collect::<Vec<_>>();
                    entry.with_extra_chunks(extra_chunks)
                } else {
                    entry
                }
            }
            Self::Apply {
                globs,
                set,
                modify,
                remove,
                platform,
            } => {
                if globs.matches_any(entry.header().path()) {
                    transform_entry(
                        entry,
                        platform,
                        set.as_ref(),
                        modify.as_ref(),
                        remove.as_ref(),
                    )
                } else {
                    entry
                }
            }
        }
    }
}

#[inline]
fn transform_entry<T>(
    entry: NormalEntry<T>,
    platform: &AcePlatform,
    set: Option<&AclEntries>,
    modify: Option<&AclEntries>,
    remove: Option<&AclEntries>,
) -> NormalEntry<T>
where
    T: Clone,
    RawChunk<T>: Chunk,
    RawChunk<T>: From<RawChunk>,
{
    let extra_without_known = entry
        .extra_chunks()
        .iter()
        .filter(|it| it.ty() != crate::chunk::faCe && it.ty() != crate::chunk::faCl)
        .cloned();
    let acls = entry.acl().unwrap_or_default();
    let acls = transform_acl(acls, platform, set, modify, remove);
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
    entry.with_extra_chunks(extra_chunks)
}

fn transform_acl(
    mut acls: Acls,
    platform: &AcePlatform,
    set: Option<&AclEntries>,
    modify: Option<&AclEntries>,
    remove: Option<&AclEntries>,
) -> Acls {
    let acl = acls.entry(platform.clone()).or_default();

    if let Some(set) = set {
        let ace = set.to_ace();
        log::debug!("Setting ace {ace}");
        acl.clear();
        acl.push(ace);
    }
    if let Some(modify) = modify {
        let ace = modify.to_ace();
        let item = acl.iter_mut().find(|it| modify.is_match(it));
        if let Some(item) = item {
            log::debug!("Modifying ace {item} to {ace}");
            item.permission = ace.permission;
        } else {
            log::debug!("Adding ace {ace} ");
            acl.push(ace);
        }
    }
    if let Some(remove) = remove {
        log::debug!("Removing ace {}", remove.to_ace());
        acl.retain(|it| !remove.is_match(it));
    }
    acls
}

fn parse_acl_dump(reader: impl io::BufRead) -> io::Result<HashMap<String, Acls>> {
    let mut result = HashMap::new();
    let mut current_file = None;
    let mut current_platform = AcePlatform::General;
    let lines = reader.lines();

    for line in lines {
        let line = line?;
        if line.is_empty() {
            // ignore
            continue;
        }
        if let Some(path) = line.strip_prefix("# file: ") {
            current_file = Some(String::from(path));
        } else if line.starts_with("# owner: ") || line.starts_with("# group: ") {
            // ignore
            continue;
        } else if let Some(platform) = line.strip_prefix("# platform: ") {
            current_platform = AcePlatform::from_str(platform).expect("Infallible error occurred");
        } else if let Some(file) = &current_file {
            let ace =
                Ace::from_str(&line).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            let file_entry = result.entry(file.clone()).or_insert_with(Acls::new);
            file_entry
                .entry(current_platform.clone())
                .or_default()
                .push(ace);
        }
    }
    Ok(result)
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
                allow: None,
                owner: OwnerType::User(Identifier("uname".into())),
                permissions: None,
            }
        );
        assert_eq!(
            AclEntries::from_str("user:").unwrap(),
            AclEntries {
                default: false,
                allow: None,
                owner: OwnerType::Owner,
                permissions: None,
            }
        );
        assert_eq!(
            AclEntries::from_str("uname:").unwrap(),
            AclEntries {
                default: false,
                allow: None,
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
                allow: None,
                owner: OwnerType::OwnerGroup,
                permissions: None,
            }
        );
        assert_eq!(
            AclEntries::from_str("group:gname").unwrap(),
            AclEntries {
                default: false,
                allow: None,
                owner: OwnerType::Group(Identifier("gname".into())),
                permissions: None,
            }
        );
        assert_eq!(
            AclEntries::from_str("g::").unwrap(),
            AclEntries {
                default: false,
                allow: None,
                owner: OwnerType::OwnerGroup,
                permissions: Some(Vec::new()),
            }
        );
        assert_eq!(
            AclEntries::from_str("group:gname:").unwrap(),
            AclEntries {
                default: false,
                allow: None,
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
                allow: None,
                owner: OwnerType::Mask,
                permissions: None,
            }
        );
        assert_eq!(
            AclEntries::from_str("mask:").unwrap(),
            AclEntries {
                default: false,
                allow: None,
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
                allow: None,
                owner: OwnerType::Other,
                permissions: None,
            }
        );
        assert_eq!(
            AclEntries::from_str("other:").unwrap(),
            AclEntries {
                default: false,
                allow: None,
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
                allow: None,
                owner: OwnerType::Owner,
                permissions: Some(vec!["r".into(), "w".into(), "x".into()]),
            }
        );
        assert_eq!(
            AclEntries::from_str("d:u::rw-").unwrap(),
            AclEntries {
                default: true,
                allow: None,
                owner: OwnerType::Owner,
                permissions: Some(vec!["r".into(), "w".into()]),
            }
        );
        assert_eq!(
            AclEntries::from_str("d:u::r-x").unwrap(),
            AclEntries {
                default: true,
                allow: None,
                owner: OwnerType::Owner,
                permissions: Some(vec!["r".into(), "x".into()]),
            }
        );
        assert_eq!(
            AclEntries::from_str("d:u::-w-").unwrap(),
            AclEntries {
                default: true,
                allow: None,
                owner: OwnerType::Owner,
                permissions: Some(vec!["w".into()]),
            }
        );
        assert_eq!(
            AclEntries::from_str("d:u::---").unwrap(),
            AclEntries {
                default: true,
                allow: None,
                owner: OwnerType::Owner,
                permissions: Some(vec![]),
            }
        );
    }

    #[test]
    fn transform_acl_set() {
        let mut acls = Acls::new();
        acls.insert(
            AcePlatform::Linux,
            vec![Ace {
                flags: Flag::empty(),
                owner_type: OwnerType::Owner,
                allow: true,
                permission: Permission::READ,
            }],
        );

        let actual = transform_acl(
            acls,
            &AcePlatform::Linux,
            Some(&AclEntries::from_str("u::rw-").unwrap()),
            None,
            None,
        );
        let expected = {
            let mut acls = Acls::new();
            acls.insert(
                AcePlatform::Linux,
                vec![Ace {
                    flags: Flag::empty(),
                    owner_type: OwnerType::Owner,
                    allow: true,
                    permission: Permission::READ | Permission::WRITE,
                }],
            );
            acls
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn transform_acl_add() {
        let acls = Acls::new();
        let actual = transform_acl(
            acls,
            &AcePlatform::Linux,
            None,
            Some(&AclEntries::from_str("u::rw-").unwrap()),
            None,
        );
        let expected = {
            let mut acls = Acls::new();
            acls.insert(
                AcePlatform::Linux,
                vec![Ace {
                    flags: Flag::empty(),
                    owner_type: OwnerType::Owner,
                    allow: true,
                    permission: Permission::READ | Permission::WRITE,
                }],
            );
            acls
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn transform_acl_modify() {
        let mut acls = Acls::new();
        acls.insert(
            AcePlatform::Linux,
            vec![Ace {
                flags: Flag::empty(),
                owner_type: OwnerType::Owner,
                allow: true,
                permission: Permission::READ,
            }],
        );
        let actual = transform_acl(
            acls,
            &AcePlatform::Linux,
            None,
            Some(&AclEntries::from_str("u::rwx").unwrap()),
            None,
        );
        let expected = {
            let mut acls = Acls::new();
            acls.insert(
                AcePlatform::Linux,
                vec![Ace {
                    flags: Flag::empty(),
                    owner_type: OwnerType::Owner,
                    allow: true,
                    permission: Permission::READ | Permission::WRITE | Permission::EXECUTE,
                }],
            );
            acls
        };

        assert_eq!(actual, expected);
    }

    #[test]
    fn transform_acl_remove() {
        let mut acls = Acls::new();
        acls.insert(
            AcePlatform::Linux,
            vec![
                Ace {
                    flags: Flag::empty(),
                    owner_type: OwnerType::Owner,
                    allow: true,
                    permission: Permission::READ | Permission::WRITE,
                },
                Ace {
                    flags: Flag::empty(),
                    owner_type: OwnerType::User(Identifier("test".into())),
                    allow: true,
                    permission: Permission::READ,
                },
            ],
        );
        let actual = transform_acl(
            acls,
            &AcePlatform::Linux,
            None,
            None,
            Some(&AclEntries::from_str("u:").unwrap()),
        );
        let expected = {
            let mut acls = Acls::new();
            acls.insert(
                AcePlatform::Linux,
                vec![Ace {
                    flags: Flag::empty(),
                    owner_type: OwnerType::User(Identifier("test".into())),
                    allow: true,
                    permission: Permission::READ,
                }],
            );
            acls
        };
        assert_eq!(actual, expected);
    }
}
