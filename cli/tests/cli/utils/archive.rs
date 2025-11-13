use pna::prelude::*;
use std::{io, path::Path};

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
    let password = password.into();
    let mut archive = pna::Archive::open(path)?;
    let entries = archive
        .entries()
        .extract_solid_entries(password.map(|p| p.as_bytes()));
    for entry in entries {
        f(entry?);
    }
    Ok(())
}
