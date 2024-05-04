use crate::{
    cli::{FileArgs, PasswordArgs, Verbosity},
    command::{
        ask_password,
        commons::{run_process_archive_reader, KeepOptions},
        Command,
    },
    utils::{self, with_part_n, GlobPatterns},
};
use clap::{Parser, ValueHint};
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
    io::{self, prelude::*},
    path::{Path, PathBuf},
    time::{Instant, SystemTime},
};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
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
    let file = File::open(&args.file.archive)?;
    run_extract_archive_reader(
        file,
        args.file.files,
        || password.as_deref(),
        |i| File::open(with_part_n(&args.file.archive, i).unwrap()),
        OutputOption {
            overwrite: args.overwrite,
            out_dir: args.out_dir,
            keep_options,
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
}

pub(crate) fn run_extract_archive_reader<'p, R, Provider, N>(
    reader: R,
    files: Vec<PathBuf>,
    mut password_provider: Provider,
    get_next_reader: N,
    args: OutputOption,
    verbosity: Verbosity,
) -> io::Result<()>
where
    R: Read,
    Provider: FnMut() -> Option<&'p str>,
    N: FnMut(usize) -> io::Result<R>,
{
    let password = password_provider().map(ToOwned::to_owned);
    let globs = GlobPatterns::new(files.iter().map(|p| p.to_string_lossy()))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let mut hard_link_entries = Vec::new();

    let (tx, rx) = std::sync::mpsc::channel();
    run_process_archive_reader(
        reader,
        password_provider,
        |entry| {
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
            pool.spawn_fifo(move || {
                tx.send(extract_entry(
                    item,
                    password,
                    args.overwrite,
                    out_dir.as_deref(),
                    args.keep_options,
                    verbosity,
                ))
                .unwrap_or_else(|e| panic!("{e}: {}", item_path.display()));
            });
            Ok(())
        },
        get_next_reader,
    )?;
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
        item.metadata().permission().and_then(permissions)
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
fn permissions(_: &Permission) -> Option<((), (), ())> {
    None
}
#[cfg(unix)]
fn permissions(permission: &Permission) -> Option<(Permissions, Option<User>, Option<Group>)> {
    Some((
        Permissions::from_mode(permission.permissions().into()),
        search_owner(permission.uname(), permission.uid()),
        search_group(permission.gname(), permission.gid()),
    ))
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
