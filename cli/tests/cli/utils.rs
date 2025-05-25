pub mod archive;
pub mod diff;

use std::{
    fs, io,
    path::{Component, Path},
};

#[derive(rust_embed::Embed)]
#[folder = "../resources/test"]
pub struct TestResources;

impl TestResources {
    pub fn extract_all(into: impl AsRef<Path>) -> io::Result<()> {
        extract_all::<Self>(into)
    }
    pub fn extract_in(item: &str, into: impl AsRef<Path>) -> io::Result<()> {
        extract_in::<Self>(item, into)
    }
}

#[derive(rust_embed::Embed)]
#[folder = "../lib"]
pub struct LibSourceCode;

impl LibSourceCode {
    pub fn extract_all(into: impl AsRef<Path>) -> io::Result<()> {
        extract_all::<Self>(into)
    }
    pub fn extract_in(item: &str, into: impl AsRef<Path>) -> io::Result<()> {
        extract_in::<Self>(item, into)
    }
}

pub fn extract_all<T: rust_embed::Embed>(into: impl AsRef<Path>) -> io::Result<()> {
    let path = into.as_ref();
    T::iter().try_for_each(|i| {
        let path = path.join(i.as_ref());
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, T::get(&i).unwrap().data)
    })
}

pub fn extract_in<T: rust_embed::Embed>(item: &str, into: impl AsRef<Path>) -> io::Result<()> {
    let path = into.as_ref();
    if let Some(b) = T::get(item) {
        let path = path.join(item);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, b.data)?;
        return Ok(());
    }
    T::iter().try_for_each(|i| {
        if i.starts_with(item) {
            let path = path.join(i.as_ref());
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, T::get(&i).unwrap().data)?;
        }
        Ok(())
    })
}

pub fn setup() {
    fs::create_dir_all(env!("CARGO_TARGET_TMPDIR")).expect("Failed to create working dir");
    std::env::set_current_dir(env!("CARGO_TARGET_TMPDIR")).expect("Failed to set current dir");
}

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

pub fn remove_with_empty_parents(path: impl AsRef<Path>) -> io::Result<()> {
    fn inner(path: &Path) -> io::Result<()> {
        pna::fs::remove_path_all(path)?;
        let mut current_path = path;
        while let Some(dir) = current_path.parent() {
            if fs::read_dir(dir)?.next().is_none() {
                fs::remove_dir(dir)?;
                current_path = dir;
            } else {
                break;
            }
        }
        Ok(())
    }
    inner(path.as_ref())
}

pub fn components_count<P: AsRef<Path>>(p: P) -> usize {
    p.as_ref()
        .components()
        .filter(|it| matches!(it, Component::Normal(_)))
        .count()
}
