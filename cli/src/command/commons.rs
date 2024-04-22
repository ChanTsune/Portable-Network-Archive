use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs},
    utils::part_name,
};
#[cfg(unix)]
use nix::unistd::{Group, User};
use pna::{
    Archive, EntryBuilder, EntryName, EntryPart, EntryReference, ExtendedAttribute, Permission,
    RegularEntry, WriteOption,
};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::{
    fs,
    io::{self, prelude::*},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct KeepOptions {
    pub(crate) keep_timestamp: bool,
    pub(crate) keep_permission: bool,
    pub(crate) keep_xattr: bool,
}

pub(crate) fn collect_items(
    files: &[PathBuf],
    recursive: bool,
    keep_dir: bool,
    exclude: &Option<Vec<PathBuf>>,
) -> io::Result<Vec<PathBuf>> {
    let exclude = exclude.as_ref().map(|it| {
        it.iter()
            .filter_map(|path| path.canonicalize().ok())
            .collect::<Vec<_>>()
    });
    fn inner(
        result: &mut Vec<PathBuf>,
        path: &Path,
        recursive: bool,
        keep_dir: bool,
        exclude: Option<&Vec<PathBuf>>,
    ) -> io::Result<()> {
        let cpath = path.canonicalize()?;
        if let Some(exclude) = exclude {
            if exclude.iter().any(|it| it.eq(&cpath)) {
                return Ok(());
            }
        }
        if path.is_dir() {
            if keep_dir {
                result.push(path.to_path_buf());
            }
            if recursive {
                for p in fs::read_dir(path)? {
                    inner(result, &p?.path(), recursive, keep_dir, exclude)?;
                }
            }
        } else if path.is_file() {
            result.push(path.to_path_buf());
        }
        Ok(())
    }
    let mut target_items = vec![];
    for p in files {
        inner(&mut target_items, p, recursive, keep_dir, exclude.as_ref())?;
    }
    Ok(target_items)
}

pub(crate) fn create_entry(
    path: &Path,
    option: WriteOption,
    keep_options: KeepOptions,
) -> io::Result<RegularEntry> {
    if path.is_symlink() {
        let source = fs::read_link(path)?;
        let entry = EntryBuilder::new_symbolic_link(
            EntryName::from_lossy(path),
            EntryReference::from_lossy(source.as_path()),
        )?;
        return apply_metadata(entry, path, keep_options)?.build();
    } else if path.is_file() {
        let mut entry = EntryBuilder::new_file(EntryName::from_lossy(path), option)?;
        entry.write_all(&fs::read(path)?)?;
        return apply_metadata(entry, path, keep_options)?.build();
    } else if path.is_dir() {
        let entry = EntryBuilder::new_dir(EntryName::from_lossy(path));
        return apply_metadata(entry, path, keep_options)?.build();
    }
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Currently not a regular file is not supported.",
    ))
}

pub(crate) fn entry_option(
    compression: CompressionAlgorithmArgs,
    cipher: CipherAlgorithmArgs,
    password: Option<String>,
) -> WriteOption {
    let (algorithm, level) = compression.algorithm();
    let mut option_builder = WriteOption::builder();
    option_builder
        .compression(algorithm)
        .compression_level(level.unwrap_or_default())
        .encryption(if password.is_some() {
            cipher.algorithm()
        } else {
            pna::Encryption::No
        })
        .cipher_mode(cipher.mode())
        .password(password);
    option_builder.build()
}

pub(crate) fn apply_metadata(
    mut entry: EntryBuilder,
    path: &Path,
    keep_options: KeepOptions,
) -> io::Result<EntryBuilder> {
    if keep_options.keep_timestamp || keep_options.keep_permission {
        let meta = fs::metadata(path)?;
        if keep_options.keep_timestamp {
            if let Ok(c) = meta.created() {
                if let Ok(created_since_unix_epoch) = c.duration_since(UNIX_EPOCH) {
                    entry.created(created_since_unix_epoch);
                }
            }
            if let Ok(m) = meta.modified() {
                if let Ok(modified_since_unix_epoch) = m.duration_since(UNIX_EPOCH) {
                    entry.modified(modified_since_unix_epoch);
                }
            }
            if let Ok(a) = meta.accessed() {
                if let Ok(accessed_since_unix_epoch) = a.duration_since(UNIX_EPOCH) {
                    entry.accessed(accessed_since_unix_epoch);
                }
            }
        }
        #[cfg(unix)]
        if keep_options.keep_permission {
            let mode = meta.permissions().mode() as u16;
            let uid = meta.uid();
            let gid = meta.gid();
            let user = User::from_uid(uid.into())?.unwrap();
            let group = Group::from_gid(gid.into())?.unwrap();
            entry.permission(Permission::new(
                uid.into(),
                user.name,
                gid.into(),
                group.name,
                mode,
            ));
        }
    }
    #[cfg(unix)]
    if keep_options.keep_xattr {
        if xattr::SUPPORTED_PLATFORM {
            let xattrs = xattr::list(path)?;
            for name in xattrs {
                let value = xattr::get(path, &name)?.unwrap_or_default();
                entry.add_xattr(ExtendedAttribute::new(name.to_string_lossy().into(), value));
            }
        } else {
            eprintln!("Currently extended attribute is not supported on this platform.");
        }
    }
    #[cfg(not(unix))]
    if keep_options.keep_xattr {
        eprintln!("Currently extended attribute is not supported on this platform.");
    }
    Ok(entry)
}

pub(crate) fn split_to_parts(
    mut entry_part: EntryPart,
    first: usize,
    max: usize,
) -> Vec<EntryPart> {
    let mut parts = vec![];
    let mut split_size = first;
    loop {
        match entry_part.split(split_size) {
            (write_part, Some(remaining_part)) => {
                parts.push(write_part);
                entry_part = remaining_part;
                split_size = max;
            }
            (write_part, None) => {
                parts.push(write_part);
                break;
            }
        }
    }
    parts
}

pub(crate) fn run_process_archive_reader<'p, R, Provider, F, N>(
    reader: R,
    mut password_provider: Provider,
    mut processor: F,
    mut get_next_reader: N,
) -> io::Result<()>
where
    R: Read,
    Provider: FnMut() -> Option<&'p str>,
    F: FnMut(io::Result<RegularEntry>) -> io::Result<()>,
    N: FnMut(usize) -> io::Result<R>,
{
    let mut archive = Archive::read_header(reader)?;
    let mut num_archive = 1;
    loop {
        for entry in archive.entries_with_password(password_provider()) {
            processor(entry)?;
        }
        if archive.next_archive() {
            num_archive += 1;
            let next_reader = get_next_reader(num_archive)?;
            archive = archive.read_next_archive(next_reader)?;
        } else {
            break;
        }
    }
    Ok(())
}

pub(crate) fn run_process_archive_path<'p, P, Provider, F, N, NP>(
    path: P,
    password_provider: Provider,
    processor: F,
    mut get_next_file_path: N,
) -> io::Result<()>
where
    P: AsRef<Path>,
    Provider: FnMut() -> Option<&'p str>,
    F: FnMut(io::Result<RegularEntry>) -> io::Result<()>,
    N: FnMut(&Path, usize) -> NP,
    NP: AsRef<Path>,
{
    let path = path.as_ref();
    let file = fs::File::open(path)?;
    run_process_archive_reader(file, password_provider, processor, |num_archive| {
        let next_file_path = get_next_file_path(path, num_archive);
        fs::File::open(next_file_path)
    })
}

pub(crate) fn run_process_archive<'p, P, Provider, F>(
    path: P,
    password_provider: Provider,
    processor: F,
) -> io::Result<()>
where
    P: AsRef<Path>,
    Provider: FnMut() -> Option<&'p str>,
    F: FnMut(io::Result<RegularEntry>) -> io::Result<()>,
{
    run_process_archive_path(path, password_provider, processor, |path, n| {
        part_name(path, n).unwrap()
    })
}
