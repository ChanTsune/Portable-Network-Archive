use pna::ReadOptions;
use pna::prelude::*;
use std::{
    fs::File,
    io::{self, Write},
    mem,
    path::Path,
};

/// Definition for creating a file entry with specific permissions
pub struct FileEntryDef<'a> {
    pub path: &'a str,
    pub content: &'a [u8],
    pub permission: u16,
}

fn metadata_with_mode(mode: u16) -> pna::Metadata {
    pna::Metadata::new()
        .with_owner_uid(Some(pna::OwnerUid::from(1000)))
        .with_owner_gid(Some(pna::OwnerGid::from(1000)))
        .with_owner_user_name(Some(pna::OwnerUserName::new("user").unwrap()))
        .with_owner_group_name(Some(pna::OwnerGroupName::new("group").unwrap()))
        .with_permission_mode(Some(pna::PermissionMode::from(mode)))
}

/// Constructs an [`pna::ExtendedAttribute`] from raw name/value, panicking on
/// length-bound violations. Test-only helper; production code must propagate
/// the [`pna::LengthExceeded`] error instead.
pub fn xattr(name: &str, value: &[u8]) -> pna::ExtendedAttribute {
    pna::ExtendedAttribute::new(
        pna::XattrName::try_from(name).expect("xattr name fits within u32::MAX bytes"),
        pna::XattrValue::try_from(value).expect("xattr value fits within u32::MAX bytes"),
    )
}

/// Creates an archive with file entries having specific permissions.
/// This bypasses filesystem permission requirements by constructing entries programmatically.
pub fn create_archive_with_permissions(
    archive_path: impl AsRef<Path>,
    entries: &[FileEntryDef],
) -> io::Result<()> {
    let file = File::create(archive_path)?;
    let mut archive = pna::Archive::write_header(file)?;

    for entry_def in entries {
        archive.write_file(
            entry_def.path.into(),
            metadata_with_mode(entry_def.permission),
            pna::WriteOptions::store(),
            |writer| writer.write_all(entry_def.content),
        )?;
    }

    archive.finalize()?;
    Ok(())
}

/// Creates a solid archive with file entries having specific permissions.
pub fn create_solid_archive_with_permissions(
    archive_path: impl AsRef<Path>,
    entries: &[FileEntryDef],
) -> io::Result<()> {
    let file = File::create(archive_path)?;
    let mut archive = pna::Archive::write_solid_header(file, pna::WriteOptions::store())?;
    for entry_def in entries {
        archive.write_file(
            entry_def.path.into(),
            metadata_with_mode(entry_def.permission),
            |writer| writer.write_all(entry_def.content),
        )?;
    }

    archive.finalize()?;
    Ok(())
}

/// Creates an encrypted solid archive with file entries having specific permissions.
pub fn create_encrypted_solid_archive_with_permissions(
    archive_path: impl AsRef<Path>,
    entries: &[FileEntryDef],
    password: &str,
) -> io::Result<()> {
    let file = File::create(archive_path)?;
    let write_options = pna::WriteOptions::builder()
        .password(Some(password))
        .encryption(pna::Encryption::AES)
        .cipher_mode(pna::CipherMode::GCM)
        .build();

    let mut archive = pna::Archive::write_solid_header(file, write_options)?;
    for entry_def in entries {
        archive.write_file(
            entry_def.path.into(),
            metadata_with_mode(entry_def.permission),
            |writer| writer.write_all(entry_def.content),
        )?;
    }

    archive.finalize()?;
    Ok(())
}

/// Creates an encrypted archive with file entries having specific permissions.
pub fn create_encrypted_archive_with_permissions(
    archive_path: impl AsRef<Path>,
    entries: &[FileEntryDef],
    password: &str,
) -> io::Result<()> {
    let file = File::create(archive_path)?;
    let mut archive = pna::Archive::write_header(file)?;

    let write_options = pna::WriteOptions::builder()
        .password(Some(password))
        .encryption(pna::Encryption::AES)
        .cipher_mode(pna::CipherMode::CTR)
        .build();

    for entry_def in entries {
        archive.write_file(
            entry_def.path.into(),
            metadata_with_mode(entry_def.permission),
            write_options.clone(),
            |writer| writer.write_all(entry_def.content),
        )?;
    }

    archive.finalize()?;
    Ok(())
}

pub fn for_each_entry<F>(path: impl AsRef<Path>, f: F) -> io::Result<()>
where
    F: FnMut(pna::NormalEntry),
{
    for_each_entry_with_password(path, None, f)
}

pub fn for_each_entry_with_password<'a, F>(
    path: impl AsRef<Path>,
    password: impl Into<Option<&'a str>>,
    mut f: F,
) -> io::Result<()>
where
    F: FnMut(pna::NormalEntry),
{
    let password = password.into().map(|p| p.as_bytes());
    let mut archive = pna::Archive::open(path)?;
    let read_options = ReadOptions::with_password(password);
    let entries = archive.entries_with_options(&read_options);
    for entry in entries {
        f(entry?);
    }
    Ok(())
}

/// Entries that carry at least one xattr, sorted by entry path (directory
/// traversal order differs between filesystems).
pub fn xattrs_by_entry(
    path: impl AsRef<Path>,
    password: Option<&str>,
) -> Vec<(String, Vec<pna::ExtendedAttribute>)> {
    let mut out = Vec::new();
    for_each_entry_with_password(path, password, |e| {
        if !e.metadata().xattrs().is_empty() {
            out.push((
                e.header().path().to_string(),
                e.metadata().xattrs().to_vec(),
            ));
        }
    })
    .unwrap();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

pub fn read_symlink_target(entry: &pna::NormalEntry) -> String {
    let pna::EntryContent::SymbolicLink(target) = entry
        .content(pna::ReadOptions::with_password::<&[u8]>(None))
        .unwrap()
    else {
        panic!("entry should contain a symbolic link target");
    };
    target.to_string()
}

/// Creates a simple archive with named text entries.
pub fn create_test_archive(path: impl AsRef<Path>, entries: &[(&str, &str)]) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let file = File::create(path).unwrap();
    let mut writer = pna::Archive::write_header(file).unwrap();
    for (name, contents) in entries {
        writer
            .write_file(
                (*name).into(),
                pna::Metadata::new(),
                pna::WriteOptions::store(),
                |entry| entry.write_all(contents.as_bytes()),
            )
            .unwrap();
    }
    writer.finalize().unwrap();
}

/// Definition for creating a symlink entry with optional metadata
pub struct SymlinkEntryDef<'a> {
    pub path: &'a str,
    pub target: &'a str,
    pub permission: Option<u16>,
    pub modified: Option<pna::Duration>,
    pub accessed: Option<pna::Duration>,
    pub created: Option<pna::Duration>,
    pub link_target_type: Option<pna::LinkTargetType>,
}

/// Creates an archive containing both file and symlink entries with specific metadata.
/// This bypasses filesystem requirements by constructing entries programmatically.
pub fn create_archive_with_symlinks(
    archive_path: impl AsRef<Path>,
    file_entries: &[FileEntryDef],
    symlink_entries: &[SymlinkEntryDef],
) -> io::Result<()> {
    let file = File::create(archive_path)?;
    let mut archive = pna::Archive::write_header(file)?;

    for entry_def in file_entries {
        archive.write_file(
            entry_def.path.into(),
            metadata_with_mode(entry_def.permission),
            pna::WriteOptions::store(),
            |writer| writer.write_all(entry_def.content),
        )?;
    }

    for symlink_def in symlink_entries {
        let mut builder =
            pna::SymlinkEntryBuilder::new(symlink_def.path.into(), symlink_def.target.into())?;
        let mut metadata = pna::Metadata::new().with_link_target_type(symlink_def.link_target_type);
        if let Some(mode) = symlink_def.permission {
            metadata = metadata
                .with_owner_uid(Some(pna::OwnerUid::from(1000)))
                .with_owner_gid(Some(pna::OwnerGid::from(1000)))
                .with_owner_user_name(Some(pna::OwnerUserName::new("user").unwrap()))
                .with_owner_group_name(Some(pna::OwnerGroupName::new("group").unwrap()))
                .with_permission_mode(Some(pna::PermissionMode::from(mode)));
        }
        if let Some(m) = symlink_def.modified {
            metadata = metadata.with_modified(Some(m));
        }
        if let Some(a) = symlink_def.accessed {
            metadata = metadata.with_accessed(Some(a));
        }
        if let Some(c) = symlink_def.created {
            metadata = metadata.with_created(Some(c));
        }
        builder.metadata(metadata);
        let entry = builder.build()?;
        archive.add_entry(entry)?;
    }

    archive.finalize()?;
    Ok(())
}

/// Collects all entry names from an archive.
pub fn get_archive_entry_names(path: impl AsRef<Path>) -> Vec<String> {
    let mut names = Vec::new();
    for_each_entry(path, |entry| {
        names.push(entry.header().path().to_string());
    })
    .unwrap();
    names
}

/// Flips one byte in the data field of the first chunk of `target` type.
/// With `recompute_crc: false` the stored CRC no longer matches (CRC-level
/// corruption); with `true` the CRC is updated so the corruption is only
/// detectable by decoding the data stream.
/// Returns whether a matching non-empty chunk was found and corrupted.
pub fn corrupt_first_chunk(
    path: impl AsRef<Path>,
    target: pna::ChunkType,
    recompute_crc: bool,
) -> io::Result<bool> {
    let mut bytes = std::fs::read(&path)?;
    let target_data = {
        let mut chunk_start = pna::PNA_SIGNATURE.len();
        let mut target_data = None;
        for chunk in pna::read_chunks_from_slice(&bytes)? {
            let chunk = chunk?;
            let data_len = chunk.data().len();
            if chunk.ty() == target && data_len > 0 {
                let data_start = chunk_start + mem::size_of::<u32>() + target.len();
                target_data = Some((data_start, data_len));
                break;
            }
            chunk_start += pna::MIN_CHUNK_BYTES_SIZE + data_len;
        }
        target_data
    };

    let Some((data_start, data_len)) = target_data else {
        return Ok(false);
    };
    bytes[data_start] ^= 0xFF;
    if recompute_crc {
        let crc = (target, &bytes[data_start..data_start + data_len]).crc();
        let crc_pos = data_start + data_len;
        bytes[crc_pos..crc_pos + mem::size_of::<u32>()].copy_from_slice(&crc.to_be_bytes());
    }
    std::fs::write(&path, bytes)?;
    Ok(true)
}
