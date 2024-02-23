use crate::{
    cli::{ExtractArgs, Verbosity},
    command::{ask_password, Command, Let},
    utils::{self, part_name},
};
use glob::Pattern;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
#[cfg(unix)]
use nix::unistd::{chown, Group, User};
use pna::{Archive, DataKind, Permission, ReadOption, RegularEntry};
use rayon::{prelude::*, ThreadPoolBuilder};
use std::ops::Add;
#[cfg(target_os = "macos")]
use std::os::macos::fs::FileTimesExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(windows)]
use std::os::windows::fs::FileTimesExt;
use std::{
    fs::{self, File, Permissions},
    io,
    path::{Path, PathBuf},
    time::{Instant, SystemTime},
};

impl Command for ExtractArgs {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        extract_archive(self, verbosity)
    }
}
fn extract_archive(args: ExtractArgs, verbosity: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    let start = Instant::now();
    if verbosity != Verbosity::Quite {
        eprintln!("Extract archive {}", args.file.archive.display());
    }
    let globs = args
        .file
        .files
        .par_iter()
        .map(|p| Pattern::new(&p.to_string_lossy()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let progress_bar = if verbosity != Verbosity::Quite {
        Some(ProgressBar::new(0).with_style(ProgressStyle::default_bar().progress_chars("=> ")))
    } else {
        None
    };

    let mut hard_link_entries = Vec::new();

    let (tx, rx) = std::sync::mpsc::channel();
    run_extract(
        &args.file.archive,
        password.as_deref(),
        |entry| {
            let item = entry?;
            let item_path = PathBuf::from(item.header().path().as_str());
            if !globs.is_empty() && !globs.par_iter().any(|glob| glob.matches_path(&item_path)) {
                if verbosity == Verbosity::Verbose {
                    eprintln!("Skip: {}", item.header().path())
                }
                return Ok(());
            }
            progress_bar.let_ref(|pb| pb.inc_length(1));
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
                    args.keep_timestamp,
                    args.keep_permission,
                    verbosity,
                ))
                .unwrap_or_else(|e| panic!("{e}: {}", item_path.display()));
            });
            Ok(())
        },
        |path, num| {
            let next_file_path = part_name(path, num).unwrap();
            if verbosity == Verbosity::Verbose {
                eprintln!("Detect split: search {}", next_file_path.display());
            }
            next_file_path
        },
    )?;
    drop(tx);
    for result in rx {
        result?;
        progress_bar.let_ref(|pb| pb.inc(1));
    }

    for item in hard_link_entries {
        extract_entry(
            item,
            password.clone(),
            args.overwrite,
            args.out_dir.as_deref(),
            args.keep_timestamp,
            args.keep_permission,
            verbosity,
        )?;
        progress_bar.let_ref(|pb| pb.inc(1));
    }

    progress_bar.let_ref(|pb| pb.finish_and_clear());

    if verbosity != Verbosity::Quite {
        eprintln!(
            "Successfully extracted an archive in {}",
            HumanDuration(start.elapsed())
        );
    }
    Ok(())
}

fn run_extract<P, F, N, NP>(
    path: P,
    password: Option<&str>,
    mut extractor: F,
    mut get_next_file_path: N,
) -> io::Result<()>
where
    P: AsRef<Path>,
    F: FnMut(io::Result<RegularEntry>) -> io::Result<()>,
    N: FnMut(&Path, usize) -> NP,
    NP: AsRef<Path>,
{
    let path = path.as_ref();
    let file = File::open(path)?;
    let mut reader = Archive::read_header(file)?;
    let mut num_archive = 1;
    loop {
        for entry in reader.entries_with_password(password) {
            extractor(entry)?;
        }
        if reader.next_archive() {
            num_archive += 1;
            let next_file_path = get_next_file_path(path, num_archive);
            let file = File::open(next_file_path)?;
            reader = reader.read_next_archive(file)?;
        } else {
            break;
        }
    }
    Ok(())
}

fn extract_entry(
    item: RegularEntry,
    password: Option<String>,
    overwrite: bool,
    out_dir: Option<&Path>,
    keep_timestamp: bool,
    keep_permission: bool,
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
    let permissions = if keep_permission {
        item.metadata().permission().and_then(permissions)
    } else {
        None
    };
    match item.header().data_kind() {
        DataKind::File => {
            let mut file = File::create(&path)?;
            if keep_timestamp {
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
            let original = PathBuf::from(io::read_to_string(reader)?);
            if overwrite && path.exists() {
                utils::fs::remove(&path)?;
            }
            utils::fs::symlink(original, &path)?;
        }
        DataKind::HardLink => {
            let reader = item.reader(ReadOption::with_password(password))?;
            let mut original = PathBuf::from(io::read_to_string(reader)?);
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
        chown(&path, u.map(|i| i.uid), g.map(|g| g.gid)).map_err(io::Error::from)?;
        fs::set_permissions(&path, p)
    });
    #[cfg(not(unix))]
    if let Some(_) = permissions {
        eprintln!("Currently permission is not supported on this platform.");
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
