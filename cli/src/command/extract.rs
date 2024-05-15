use crate::{
    cli::{FileArgs, PasswordArgs, Verbosity},
    command::{
        ask_password,
        commons::{
            run_process_archive, ArchiveProvider, KeepOptions, OwnerOptions, PathArchiveProvider,
        },
        Command,
    },
    utils::{self, GlobPatterns},
};
use clap::{ArgGroup, Parser, ValueHint};
use indicatif::HumanDuration;
#[cfg(unix)]
use nix::unistd::{Group, User};
use pna::{DataKind, EntryReference, Permission, ReadOption, RegularEntry};
use rayon::ThreadPoolBuilder;
use std::ops::Add;
#[cfg(target_os = "macos")]
use std::os::macos::fs::FileTimesExt;
#[cfg(unix)]
use std::os::unix::fs::{chown, PermissionsExt};
#[cfg(windows)]
use std::os::windows::fs::FileTimesExt;
use std::{
    fs::{self, File, Permissions},
    io,
    path::{Path, PathBuf},
    time::{Instant, SystemTime},
};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(
    group(ArgGroup::new("user-flag").args(["numeric_owner", "uname"])),
    group(ArgGroup::new("group-flag").args(["numeric_owner", "gname"])),
)]
pub(crate) struct ExtractCommand {
    #[arg(long, help = "Overwrite file")]
    pub(crate) overwrite: bool,
    #[arg(long, help = "Output directory of extracted files", value_hint = ValueHint::DirPath)]
    pub(crate) out_dir: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[arg(long, help = "Restore the timestamp of the files")]
    pub(crate) keep_timestamp: bool,
    #[arg(long, help = "Restore the permissions of the files")]
    pub(crate) keep_permission: bool,
    #[arg(long, help = "Restore the extended attributes of the files")]
    pub(crate) keep_xattr: bool,
    #[arg(long, help = "Restore user from given name")]
    pub(crate) uname: Option<String>,
    #[arg(long, help = "Restore group from given name")]
    pub(crate) gname: Option<String>,
    #[arg(
        long,
        help = "Overrides the user id in the archive; the user name in the archive will be ignored"
    )]
    pub(crate) uid: Option<u32>,
    #[arg(
        long,
        help = "Overrides the group id in the archive; the group name in the archive will be ignored"
    )]
    pub(crate) gid: Option<u32>,
    #[arg(
        long,
        help = "This is equivalent to --uname \"\" --gname \"\". It causes user and group names in the archive to be ignored in favor of the numeric user and group ids."
    )]
    pub(crate) numeric_owner: bool,
    #[command(flatten)]
    pub(crate) file: FileArgs,
}

impl Command for ExtractCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        extract_archive(self, verbosity)
    }
}
fn extract_archive(args: ExtractCommand, verbosity: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    let start = Instant::now();
    if verbosity != Verbosity::Quite {
        eprintln!("Extract archive {}", args.file.archive.display());
    }
    let keep_options = KeepOptions {
        keep_timestamp: args.keep_timestamp,
        keep_permission: args.keep_permission,
        keep_xattr: args.keep_xattr,
    };
    let owner_options = OwnerOptions {
        uname: if args.numeric_owner {
            Some("".to_string())
        } else {
            args.uname
        },
        gname: if args.numeric_owner {
            Some("".to_string())
        } else {
            args.gname
        },
        uid: args.uid,
        gid: args.gid,
    };
    run_extract_archive_reader(
        PathArchiveProvider::new(&args.file.archive),
        args.file.files,
        || password.as_deref(),
        OutputOption {
            overwrite: args.overwrite,
            out_dir: args.out_dir,
            keep_options,
            owner_options,
        },
        verbosity,
    )?;
    if verbosity != Verbosity::Quite {
        eprintln!(
            "Successfully extracted an archive in {}",
            HumanDuration(start.elapsed())
        );
    }
    Ok(())
}

pub(crate) struct OutputOption {
    pub(crate) overwrite: bool,
    pub(crate) out_dir: Option<PathBuf>,
    pub(crate) keep_options: KeepOptions,
    pub(crate) owner_options: OwnerOptions,
}

pub(crate) fn run_extract_archive_reader<'p, Provider>(
    reader: impl ArchiveProvider,
    files: Vec<String>,
    mut password_provider: Provider,
    args: OutputOption,
    verbosity: Verbosity,
) -> io::Result<()>
where
    Provider: FnMut() -> Option<&'p str>,
{
    let password = password_provider().map(ToOwned::to_owned);
    let globs =
        GlobPatterns::new(files).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let mut hard_link_entries = Vec::new();

    let (tx, rx) = std::sync::mpsc::channel();
    run_process_archive(reader, password_provider, |entry| {
        let item = entry?;
        let item_path = PathBuf::from(item.header().path().as_str());
        if !globs.is_empty() && !globs.matches_any_path(&item_path) {
            if verbosity == Verbosity::Verbose {
                eprintln!("Skip: {}", item.header().path())
            }
            return Ok(());
        }
        if item.header().data_kind() == DataKind::HardLink {
            hard_link_entries.push(item);
            return Ok(());
        }
        let tx = tx.clone();
        let password = password.clone();
        let out_dir = args.out_dir.clone();
        let owner_options = args.owner_options.clone();
        pool.spawn_fifo(move || {
            tx.send(extract_entry(
                item,
                password,
                args.overwrite,
                out_dir.as_deref(),
                args.keep_options,
                owner_options,
                verbosity,
            ))
            .unwrap_or_else(|e| panic!("{e}: {}", item_path.display()));
        });
        Ok(())
    })?;
    drop(tx);
    for result in rx {
        result?;
    }

    for item in hard_link_entries {
        extract_entry(
            item,
            password.clone(),
            args.overwrite,
            args.out_dir.as_deref(),
            args.keep_options,
            args.owner_options.clone(),
            verbosity,
        )?;
    }
    Ok(())
}

pub(crate) fn extract_entry(
    item: RegularEntry,
    password: Option<String>,
    overwrite: bool,
    out_dir: Option<&Path>,
    keep_options: KeepOptions,
    owner_options: OwnerOptions,
    verbosity: Verbosity,
) -> io::Result<()> {
    let item_path = item.header().path().as_path();
    if verbosity == Verbosity::Verbose {
        eprintln!("Extract: {}", item_path.display());
    }
    let path = if let Some(out_dir) = &out_dir {
        out_dir.join(item_path)
    } else {
        item_path.to_path_buf()
    };
    if path.exists() && !overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is already exists", path.display()),
        ));
    }
    if verbosity == Verbosity::Verbose {
        eprintln!("start: {}", path.display())
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let permissions = if keep_options.keep_permission {
        item.metadata()
            .permission()
            .and_then(|p| permissions(p, &owner_options))
    } else {
        None
    };
    match item.header().data_kind() {
        DataKind::File => {
            let mut file = File::create(&path)?;
            if keep_options.keep_timestamp {
                let mut times = fs::FileTimes::new();
                if let Some(accessed) = item.metadata().accessed() {
                    times = times.set_accessed(SystemTime::UNIX_EPOCH.add(accessed));
                }
                if let Some(modified) = item.metadata().modified() {
                    times = times.set_modified(SystemTime::UNIX_EPOCH.add(modified));
                }
                #[cfg(any(windows, target_os = "macos"))]
                if let Some(created) = item.metadata().created() {
                    times = times.set_created(SystemTime::UNIX_EPOCH.add(created));
                }
                file.set_times(times)?;
            }
            let mut reader = item.reader(ReadOption::with_password(password))?;
            io::copy(&mut reader, &mut file)?;
        }
        DataKind::Directory => {
            fs::create_dir_all(&path)?;
        }
        DataKind::SymbolicLink => {
            let reader = item.reader(ReadOption::with_password(password))?;
            let original = EntryReference::from_lossy(io::read_to_string(reader)?);
            if overwrite && path.exists() {
                utils::fs::remove(&path)?;
            }
            utils::fs::symlink(original.as_str(), &path)?;
        }
        DataKind::HardLink => {
            let reader = item.reader(ReadOption::with_password(password))?;
            let original = EntryReference::from_lossy(io::read_to_string(reader)?);
            let mut original = PathBuf::from(original.as_str());
            if let Some(parent) = path.parent() {
                original = parent.join(original);
            }
            if overwrite && path.exists() {
                utils::fs::remove(&path)?;
            }
            fs::hard_link(original, &path)?;
        }
    }
    #[cfg(unix)]
    permissions.map(|(p, u, g)| {
        chown(&path, u.map(|i| i.uid.as_raw()), g.map(|g| g.gid.as_raw()))?;
        fs::set_permissions(&path, p)
    });
    #[cfg(not(unix))]
    if let Some(_) = permissions {
        eprintln!("Currently permission is not supported on this platform.");
    }
    #[cfg(unix)]
    if keep_options.keep_xattr {
        if xattr::SUPPORTED_PLATFORM {
            for x in item.xattrs() {
                xattr::set(&path, x.name(), x.value())?;
            }
        } else {
            eprintln!("Currently extended attribute is not supported on this platform.");
        }
    }
    #[cfg(not(unix))]
    if keep_options.keep_xattr {
        eprintln!("Currently extended attribute is not supported on this platform.");
    }
    if verbosity == Verbosity::Verbose {
        eprintln!("end: {}", path.display());
    }
    Ok(())
}

#[cfg(not(unix))]
fn permissions(_: &Permission, _: &OwnerOptions) -> Option<((), (), ())> {
    None
}
#[cfg(unix)]
fn permissions(
    permission: &Permission,
    owner_options: &OwnerOptions,
) -> Option<(Permissions, Option<User>, Option<Group>)> {
    let p = Permissions::from_mode(permission.permissions().into());
    let user = if let Some(uid) = owner_options.uid {
        User::from_uid(uid.into()).ok().flatten()
    } else {
        search_owner(
            owner_options.uname.as_deref().unwrap_or(permission.uname()),
            permission.uid(),
        )
    };
    let group = if let Some(gid) = owner_options.gid {
        Group::from_gid(gid.into()).ok().flatten()
    } else {
        search_group(
            owner_options.gname.as_deref().unwrap_or(permission.gname()),
            permission.gid(),
        )
    };
    Some((p, user, group))
}

#[cfg(unix)]
fn search_owner(name: &str, id: u64) -> Option<User> {
    let user = User::from_name(name).ok().flatten();
    if user.is_some() {
        return user;
    }
    User::from_uid((id as u32).into()).ok().flatten()
}

#[cfg(unix)]
fn search_group(name: &str, id: u64) -> Option<Group> {
    let group = Group::from_name(name).ok().flatten();
    if group.is_some() {
        return group;
    }
    Group::from_gid((id as u32).into()).ok().flatten()
}
