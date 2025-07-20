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

pub fn for_each_entry<F>(path: impl AsRef<Path>, mut f: F) -> io::Result<()>
where
    F: FnMut(pna::NormalEntry),
{
    let mut archive = pna::Archive::open(path)?;
    let entries = archive.entries().extract_solid_entries(None);
    for entry in entries {
        f(entry?);
    }
    Ok(())
}
