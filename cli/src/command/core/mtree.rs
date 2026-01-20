//! mtree format support for @archive syntax.
//!
//! This module provides support for reading mtree manifest files via the `@file`
//! syntax, allowing users to specify files to archive with metadata overrides.

use super::{
    CreateOptions, KeepOptions, OwnerStrategy, PathFilter, PathnameEditor, TimeFilters,
    TimestampStrategy,
};
use mtree2::{Entry as MtreeEntry, FileType as MtreeFileType, MTree};
use pna::prelude::EntryBuilderExt;
use pna::{EntryBuilder, NormalEntry, WriteOptions};
use std::{
    fs::{self, Metadata},
    io::{self, Read},
    path::{Path, PathBuf},
};

/// Transforms mtree entries into archive entries.
///
/// For each entry in the mtree file:
/// 1. Reads the file from filesystem (using `contents` path if specified)
/// 2. Applies metadata overrides from mtree
/// 3. Creates a `NormalEntry`
///
/// Entries marked as `optional` are skipped if the file doesn't exist.
pub(crate) fn transform_mtree_entries<R: Read>(
    reader: R,
    create_options: &CreateOptions,
    filter: &PathFilter<'_>,
    time_filters: &TimeFilters,
) -> io::Result<Vec<io::Result<Option<NormalEntry>>>> {
    // Use empty cwd to avoid mtree2 joining paths with current working directory
    let mtree = MTree::from_reader_with_cwd(reader, PathBuf::new());
    let mut results = Vec::new();

    for entry_result in mtree {
        match entry_result {
            Ok(entry) => {
                let entry_path = entry.path();
                if filter.excluded(entry_path.to_string_lossy()) {
                    continue;
                }
                let source_path = entry.contents().unwrap_or(entry_path);
                // When nochange is set, always use filesystem mtime (bsdtar behavior)
                // Reference: libarchive's parse_file() in archive_read_support_format_mtree.c
                let mtime = if entry.no_change() {
                    fs::symlink_metadata(source_path)
                        .and_then(|m| m.modified())
                        .ok()
                } else {
                    entry.time().or_else(|| {
                        fs::symlink_metadata(source_path)
                            .and_then(|m| m.modified())
                            .ok()
                    })
                };
                // For ctime filtering, fall back to mtime since mtree never has ctime
                // This matches bsdtar behavior (time_excluded() in archive_match.c)
                if !time_filters.matches_or_inactive(mtime, mtime) {
                    continue;
                }
                match create_entry_from_mtree(&entry, create_options) {
                    Ok(Some(normal_entry)) => results.push(Ok(Some(normal_entry))),
                    Ok(None) => {}
                    Err(e) if entry.optional() && e.kind() == io::ErrorKind::NotFound => {
                        log::warn!(
                            "Skipping optional mtree entry (file not found): {}",
                            entry.path().display()
                        )
                    }
                    Err(e) => results.push(Err(io::Error::new(
                        e.kind(),
                        format!("{}: {}", entry.path().display(), e),
                    ))),
                }
            }
            Err(e) => results.push(Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("mtree parse error: {}", e),
            ))),
        }
    }

    Ok(results)
}

/// Creates a single archive entry from an mtree entry.
fn create_entry_from_mtree(
    mtree_entry: &MtreeEntry,
    CreateOptions {
        option,
        keep_options,
        pathname_editor,
    }: &CreateOptions,
) -> io::Result<Option<NormalEntry>> {
    let entry_path = mtree_entry.path();

    let Some(entry_name) = pathname_editor.edit_entry_name(entry_path) else {
        return Ok(None);
    };

    let source_path = mtree_entry.contents().unwrap_or(entry_path);

    match mtree_entry.file_type() {
        Some(MtreeFileType::File) => {
            let metadata = get_metadata(source_path)?;
            if !metadata.is_file() {
                return Err(type_mismatch_error(source_path, "file", "regular file"));
            }
            create_file_entry(
                entry_name,
                source_path,
                mtree_entry,
                &metadata,
                option,
                keep_options,
            )
        }
        None => {
            let metadata = get_metadata(source_path)?;
            if metadata.is_file() {
                create_file_entry(
                    entry_name,
                    source_path,
                    mtree_entry,
                    &metadata,
                    option,
                    keep_options,
                )
            } else if metadata.is_dir() {
                create_dir_entry(entry_name, source_path, mtree_entry, keep_options)
            } else {
                let link_meta = fs::symlink_metadata(source_path)?;
                if link_meta.file_type().is_symlink() {
                    create_symlink_entry(
                        entry_name,
                        source_path,
                        mtree_entry,
                        pathname_editor,
                        &link_meta,
                        keep_options,
                    )
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!("unsupported file type: {}", source_path.display()),
                    ))
                }
            }
        }
        Some(MtreeFileType::Directory) => {
            let link_meta = fs::symlink_metadata(source_path);
            if let Ok(ref meta) = link_meta
                && !meta.is_dir()
            {
                return Err(type_mismatch_error(source_path, "dir", "directory"));
            }
            create_dir_entry(entry_name, source_path, mtree_entry, keep_options)
        }
        Some(MtreeFileType::SymbolicLink) => {
            let metadata = fs::symlink_metadata(source_path)?;
            if !metadata.file_type().is_symlink() {
                return Err(type_mismatch_error(source_path, "link", "symlink"));
            }
            create_symlink_entry(
                entry_name,
                source_path,
                mtree_entry,
                pathname_editor,
                &metadata,
                keep_options,
            )
        }
        Some(
            MtreeFileType::BlockDevice
            | MtreeFileType::CharacterDevice
            | MtreeFileType::Fifo
            | MtreeFileType::Socket,
        ) => {
            log::warn!(
                "Skipping unsupported file type in mtree (block/char device, fifo, or socket): {}",
                entry_path.display()
            );
            Ok(None)
        }
    }
}

/// Fetches metadata for a path, formatting errors with the path for context.
fn get_metadata(path: &Path) -> io::Result<Metadata> {
    fs::metadata(path).map_err(|e| io::Error::new(e.kind(), format!("{}: {}", path.display(), e)))
}

/// Creates a type mismatch error for when mtree specifies a type that doesn't match the filesystem.
fn type_mismatch_error(path: &Path, mtree_type: &str, expected: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "{}: mtree specifies type={} but path is not a {}",
            path.display(),
            mtree_type,
            expected
        ),
    )
}

/// Creates a file entry from mtree specification.
fn create_file_entry(
    entry_name: pna::EntryName,
    source_path: &Path,
    mtree_entry: &MtreeEntry,
    metadata: &Metadata,
    option: &WriteOptions,
    keep_options: &KeepOptions,
) -> io::Result<Option<NormalEntry>> {
    let mut entry = EntryBuilder::new_file(entry_name, option)?;

    let file = fs::File::open(source_path)?;
    let mut reader = io::BufReader::with_capacity(64 * 1024, file);
    io::copy(&mut reader, &mut entry)?;

    apply_mtree_metadata(entry, mtree_entry, metadata, keep_options)?
        .build()
        .map(Some)
}

/// Creates a directory entry from mtree specification.
fn create_dir_entry(
    entry_name: pna::EntryName,
    source_path: &Path,
    mtree_entry: &MtreeEntry,
    keep_options: &KeepOptions,
) -> io::Result<Option<NormalEntry>> {
    let entry = EntryBuilder::new_dir(entry_name);

    let metadata = match fs::metadata(source_path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return apply_mtree_metadata_without_fs(entry, mtree_entry, keep_options)?
                .build()
                .map(Some);
        }
        Err(e) => return Err(e),
    };

    apply_mtree_metadata(entry, mtree_entry, &metadata, keep_options)?
        .build()
        .map(Some)
}

/// Creates a symlink entry from mtree specification.
fn create_symlink_entry(
    entry_name: pna::EntryName,
    source_path: &Path,
    mtree_entry: &MtreeEntry,
    pathname_editor: &PathnameEditor,
    metadata: &Metadata,
    keep_options: &KeepOptions,
) -> io::Result<Option<NormalEntry>> {
    let link_target = if let Some(link) = mtree_entry.link() {
        pathname_editor.edit_symlink(link)
    } else {
        pathname_editor.edit_symlink(&fs::read_link(source_path)?)
    };

    let entry = EntryBuilder::new_symlink(entry_name, link_target)?;

    apply_mtree_metadata(entry, mtree_entry, metadata, keep_options)?
        .build()
        .map(Some)
}

/// Applies metadata from mtree entry, with filesystem metadata as fallback.
/// When `nochange` keyword is set, always uses filesystem metadata (bsdtar behavior).
/// Reference: libarchive's parse_file() in archive_read_support_format_mtree.c
fn apply_mtree_metadata(
    mut entry: EntryBuilder,
    mtree_entry: &MtreeEntry,
    fs_metadata: &Metadata,
    keep_options: &KeepOptions,
) -> io::Result<EntryBuilder> {
    let nochange = mtree_entry.no_change();

    if let TimestampStrategy::Preserve {
        mtime,
        ctime,
        atime,
    } = keep_options.timestamp_strategy
    {
        let mtree_time = if nochange { None } else { mtree_entry.time() };
        if let Some(m) = mtime.resolve(mtree_time.or_else(|| fs_metadata.modified().ok())) {
            entry.modified_time(m);
        }
        if let Some(c) = ctime.resolve(fs_metadata.created().ok()) {
            entry.created_time(c);
        }
        if let Some(a) = atime.resolve(fs_metadata.accessed().ok()) {
            entry.accessed_time(a);
        }
    }

    #[cfg(unix)]
    if let OwnerStrategy::Preserve { options } = &keep_options.owner_strategy {
        use crate::utils::fs::{Group, User};
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let fs_mode = fs_metadata.permissions().mode() as u16;
        let mode: u16 = match (nochange, mtree_entry.mode()) {
            (true, _) | (false, None) => fs_mode,
            (false, Some(mtree_mode)) => u32::from(mtree_mode) as u16,
        };

        let uid = resolve_id(nochange, options.uid, mtree_entry.uid(), fs_metadata.uid());
        let gid = resolve_id(nochange, options.gid, mtree_entry.gid(), fs_metadata.gid());

        let uname = resolve_name(
            nochange,
            options.uname.as_ref(),
            mtree_entry.uname(),
            || {
                User::from_uid(uid.into())
                    .ok()
                    .and_then(|u| u.name().map(String::from))
            },
        );
        let gname = resolve_name(
            nochange,
            options.gname.as_ref(),
            mtree_entry.gname(),
            || {
                Group::from_gid(gid.into())
                    .ok()
                    .and_then(|g| g.name().map(String::from))
            },
        );

        entry.permission(pna::Permission::new(
            uid.into(),
            uname,
            gid.into(),
            gname,
            mode,
        ));
    }

    Ok(entry)
}

/// Resolves a numeric ID (uid or gid) with nochange and override handling.
#[cfg(unix)]
fn resolve_id(nochange: bool, override_id: Option<u32>, mtree_id: Option<u32>, fs_id: u32) -> u32 {
    if nochange {
        override_id.unwrap_or(fs_id)
    } else {
        override_id.or(mtree_id).unwrap_or(fs_id)
    }
}

/// Resolves a name (uname or gname) with nochange, override, and mtree handling.
#[cfg(unix)]
fn resolve_name<F>(
    nochange: bool,
    override_name: Option<&String>,
    mtree_name: Option<&[u8]>,
    lookup_from_id: F,
) -> String
where
    F: FnOnce() -> Option<String>,
{
    if let Some(name) = override_name {
        return name.clone();
    }
    if !nochange && let Some(name) = mtree_name {
        return String::from_utf8_lossy(name).into_owned();
    }
    lookup_from_id().unwrap_or_default()
}

/// Applies metadata from mtree entry only (no filesystem metadata available).
fn apply_mtree_metadata_without_fs(
    mut entry: EntryBuilder,
    mtree_entry: &MtreeEntry,
    keep_options: &KeepOptions,
) -> io::Result<EntryBuilder> {
    if let TimestampStrategy::Preserve { mtime, .. } = keep_options.timestamp_strategy
        && let Some(m) = mtime.resolve(mtree_entry.time())
    {
        entry.modified_time(m);
    }

    #[cfg(unix)]
    if let OwnerStrategy::Preserve { options } = &keep_options.owner_strategy {
        let uid = options.uid.or(mtree_entry.uid()).unwrap_or(0);
        let gid = options.gid.or(mtree_entry.gid()).unwrap_or(0);
        let mode: u16 = mtree_entry
            .mode()
            .map(|m| u32::from(m) as u16)
            .unwrap_or(0o755);

        let uname = resolve_name(false, options.uname.as_ref(), mtree_entry.uname(), || None);
        let gname = resolve_name(false, options.gname.as_ref(), mtree_entry.gname(), || None);

        entry.permission(pna::Permission::new(
            uid.into(),
            uname,
            gid.into(),
            gname,
            mode,
        ));
    }

    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn mtree_optional_keyword_is_parsed() {
        let data = b"#mtree\nfile1.txt\nfile2.txt optional\n";
        let mtree = MTree::from_reader_with_cwd(&data[..], PathBuf::new());
        let entries: Vec<_> = mtree.collect();

        assert_eq!(entries.len(), 2);
        let entry1 = entries[0].as_ref().unwrap();
        let entry2 = entries[1].as_ref().unwrap();

        assert_eq!(entry1.path().to_str(), Some("file1.txt"));
        assert!(!entry1.optional());

        assert_eq!(entry2.path().to_str(), Some("file2.txt"));
        assert!(entry2.optional(), "file2.txt should be marked as optional");
    }
}
