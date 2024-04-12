use crate::cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs};
#[cfg(unix)]
use nix::unistd::{Group, User};
use pna::{
    EntryBuilder, EntryName, EntryPart, EntryReference, Permission, RegularEntry, WriteOption,
};
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

pub(crate) fn collect_items(
    files: Vec<PathBuf>,
    recursive: bool,
    keep_dir: bool,
) -> io::Result<Vec<PathBuf>> {
    fn collect_items(
        result: &mut Vec<PathBuf>,
        path: &Path,
        recursive: bool,
        keep_dir: bool,
    ) -> io::Result<()> {
        if path.is_dir() {
            if keep_dir {
                result.push(path.to_path_buf());
            }
            if recursive {
                for p in std::fs::read_dir(path)? {
                    collect_items(result, &p?.path(), recursive, keep_dir)?;
                }
            }
        } else if path.is_file() {
            result.push(path.to_path_buf());
        }
        Ok(())
    }
    let mut target_items = vec![];
    for p in files {
        collect_items(&mut target_items, p.as_ref(), recursive, keep_dir)?;
    }
    Ok(target_items)
}

pub(crate) fn create_entry(
    path: &Path,
    option: WriteOption,
    keep_timestamp: bool,
    keep_permission: bool,
) -> io::Result<RegularEntry> {
    if path.is_symlink() {
        let source = fs::read_link(path)?;
        let entry = EntryBuilder::new_symbolic_link(
            EntryName::from_lossy(path),
            EntryReference::from_lossy(source.as_path()),
        )?;
        return apply_metadata(entry, path, keep_timestamp, keep_permission)?.build();
    } else if path.is_file() {
        let mut entry = EntryBuilder::new_file(EntryName::from_lossy(path), option)?;
        entry.write_all(&fs::read(path)?)?;
        return apply_metadata(entry, path, keep_timestamp, keep_permission)?.build();
    } else if path.is_dir() {
        let entry = EntryBuilder::new_dir(EntryName::from_lossy(path));
        return apply_metadata(entry, path, keep_timestamp, keep_permission)?.build();
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
    keep_timestamp: bool,
    keep_permission: bool,
) -> io::Result<EntryBuilder> {
    if keep_timestamp || keep_permission {
        let meta = fs::metadata(path)?;
        if keep_timestamp {
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
        if keep_permission {
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
