use crate::{
    cli::{ExtractArgs, Verbosity},
    command::{ask_password, Let},
    utils::part_name,
};
use glob::Pattern;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use libpna::{ArchiveReader, DataKind, Permission, ReadEntry, ReadOption};
#[cfg(unix)]
use nix::unistd::{chown, Group, User};
use rayon::ThreadPoolBuilder;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs::{self, File, Permissions},
    io,
    path::PathBuf,
    time::Instant,
};

pub(crate) fn extract_archive(args: ExtractArgs, verbosity: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    let start = Instant::now();
    if verbosity != Verbosity::Quite {
        println!("Extract archive {}", args.file.archive.display());
    }
    let globs = args
        .file
        .files
        .iter()
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

    let file = File::open(&args.file.archive)?;
    let mut reader = ArchiveReader::read_header(file)?;
    let mut num_archive = 1;

    let (tx, rx) = std::sync::mpsc::channel();
    loop {
        for entry in reader.entries_with_password(password.clone()) {
            let item = entry?;
            let item_path = PathBuf::from(item.header().path().as_str());
            if !globs.is_empty() && !globs.iter().any(|glob| glob.matches_path(&item_path)) {
                if verbosity == Verbosity::Verbose {
                    println!("Skip: {}", item.header().path())
                }
                continue;
            }
            progress_bar.let_ref(|pb| pb.inc_length(1));
            let tx = tx.clone();
            let password = password.clone();
            let out_dir = args.out_dir.clone();
            let keep_permission = args.keep_permission;
            pool.spawn_fifo(move || {
                tx.send(extract_entry(
                    item_path.clone(),
                    item,
                    password,
                    args.overwrite,
                    out_dir,
                    keep_permission,
                    verbosity,
                ))
                .unwrap_or_else(|e| panic!("{e}: {}", item_path.display()));
            });
        }
        if reader.next_archive() {
            num_archive += 1;
            let part_n_name = part_name(&args.file.archive, num_archive).unwrap();
            if verbosity == Verbosity::Verbose {
                println!("Detect split: search {}", part_n_name.display());
            }
            let file = File::open(part_n_name)?;
            reader = reader.read_next_archive(file)?;
        } else {
            break;
        }
    }
    drop(tx);
    for result in rx {
        result?;
        progress_bar.let_ref(|pb| pb.inc(1));
    }
    progress_bar.let_ref(|pb| pb.finish_and_clear());

    if verbosity != Verbosity::Quite {
        println!(
            "Successfully extracted an archive in {}",
            HumanDuration(start.elapsed())
        );
    }
    Ok(())
}

fn extract_entry(
    item_path: PathBuf,
    item: ReadEntry,
    password: Option<String>,
    overwrite: bool,
    out_dir: Option<PathBuf>,
    keep_permission: bool,
    verbosity: Verbosity,
) -> io::Result<()> {
    if verbosity == Verbosity::Verbose {
        println!("Extract: {}", item_path.display());
    }
    let path = if let Some(out_dir) = &out_dir {
        out_dir.join(&item_path)
    } else {
        item_path
    };
    if path.exists() && !overwrite {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is alrady exists", path.display()),
        ));
    }
    if verbosity == Verbosity::Verbose {
        println!("start: {}", path.display())
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
            let mut reader = item.into_reader({
                let mut builder = ReadOption::builder();
                if let Some(password) = password {
                    builder.password(password);
                }
                builder.build()
            })?;
            io::copy(&mut reader, &mut file)?;
        }
        DataKind::Directory => {
            fs::create_dir_all(&path)?;
        }
        DataKind::SymbolicLink => {}
        DataKind::HardLink => {}
    }
    #[cfg(unix)]
    permissions.map(|(p, u, g)| {
        chown(&path, u.map(|i| i.uid), g.map(|g| g.gid)).map_err(io::Error::from)?;
        fs::set_permissions(&path, p)
    });
    if verbosity == Verbosity::Verbose {
        println!("end: {}", path.display())
    }
    Ok(())
}

#[cfg(not(unix))]
fn permissions(permission: &Permission) -> Option<((), (), ())> {
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
