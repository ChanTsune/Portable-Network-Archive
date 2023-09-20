use crate::cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs};
use libpna::{Entry, EntryBuilder, EntryName, EntryReference, Permission, WriteOption};
#[cfg(unix)]
use nix::unistd::{Group, User};
use std::fs::metadata;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::time::UNIX_EPOCH;
use std::{
    fs, io,
    path::{Path, PathBuf},
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
) -> io::Result<impl Entry> {
    if path.is_symlink() {
        let source = fs::read_link(path)?;
        let entry = EntryBuilder::new_symbolic_link(
            EntryName::from_path_lossy(path),
            EntryReference::try_from(source.as_path()).unwrap(),
        )?;
        return apply_metadata(entry, path, keep_timestamp, keep_permission)?.build();
    } else if path.is_file() {
        let mut entry = EntryBuilder::new_file(EntryName::from_path_lossy(path), option)?;
        entry.write_all(&fs::read(path)?)?;
        return apply_metadata(entry, path, keep_timestamp, keep_permission)?.build();
    } else if path.is_dir() {
        let entry = EntryBuilder::new_dir(EntryName::from_path_lossy(path));
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
    let mut option_builder = WriteOption::builder();
    let (algorithm, level) = compression.algorithm();
    option_builder.compression(algorithm);
    if let Some(level) = level {
        option_builder.compression_level(level);
    }
    option_builder
        .encryption(if password.is_some() {
            cipher.algorithm()
        } else {
            libpna::Encryption::No
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
        let meta = metadata(path)?;
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
