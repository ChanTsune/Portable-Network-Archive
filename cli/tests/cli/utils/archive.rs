use pna::prelude::*;
use std::{
    fs::File,
    io::{self, Write},
    path::Path,
};

/// Definition for creating a file entry with specific permissions
pub struct FileEntryDef<'a> {
    pub path: &'a str,
    pub content: &'a [u8],
    pub permission: u16,
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
        let mut builder =
            pna::EntryBuilder::new_file(entry_def.path.into(), pna::WriteOptions::store())?;
        builder.permission(pna::Permission::new(
            1000,
            "user".into(),
            1000,
            "group".into(),
            entry_def.permission,
        ));
        builder.write_all(entry_def.content)?;
        let entry = builder.build()?;
        archive.add_entry(entry)?;
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
    let mut archive = pna::Archive::write_header(file)?;

    let mut solid_builder = pna::SolidEntryBuilder::new(pna::WriteOptions::store())?;
    for entry_def in entries {
        let mut builder =
            pna::EntryBuilder::new_file(entry_def.path.into(), pna::WriteOptions::store())?;
        builder.permission(pna::Permission::new(
            1000,
            "user".into(),
            1000,
            "group".into(),
            entry_def.permission,
        ));
        builder.write_all(entry_def.content)?;
        let entry = builder.build()?;
        solid_builder.add_entry(entry)?;
    }
    let solid_entry = solid_builder.build()?;
    archive.add_entry(solid_entry)?;

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
        .encryption(pna::Encryption::Aes)
        .cipher_mode(pna::CipherMode::CTR)
        .build();

    for entry_def in entries {
        let mut builder =
            pna::EntryBuilder::new_file(entry_def.path.into(), write_options.clone())?;
        builder.permission(pna::Permission::new(
            1000,
            "user".into(),
            1000,
            "group".into(),
            entry_def.permission,
        ));
        builder.write_all(entry_def.content)?;
        let entry = builder.build()?;
        archive.add_entry(entry)?;
    }

    archive.finalize()?;
    Ok(())
}

pub fn extract_single_entry(
    path: impl AsRef<Path>,
    name: &str,
) -> io::Result<Option<pna::NormalEntry>> {
    let mut archive = pna::Archive::open(path)?;
    let entries = archive.entries().extract_solid_entries(None);
    for entry in entries {
        let entry = entry?;
        if entry.header().path() == name {
            return Ok(Some(entry));
        }
    }
    Ok(None)
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
    let entries = archive.entries().extract_solid_entries(password);
    for entry in entries {
        f(entry?);
    }
    Ok(())
}
