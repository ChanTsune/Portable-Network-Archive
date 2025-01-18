pub mod diff;

use std::{
    fs, io,
    ops::Deref,
    path::{Component, Path},
};

#[derive(rust_embed::Embed)]
#[folder = "../resources/test"]
pub struct TestResources;

impl TestResources {
    pub fn extract_all(into: impl AsRef<Path>) -> io::Result<()> {
        let path = into.as_ref();
        Self::iter().try_for_each(|i| {
            let path = path.join(i.deref());
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, Self::get(&i).unwrap().data)
        })
    }

    pub fn extract_in(item: &str, into: impl AsRef<Path>) -> io::Result<()> {
        let path = into.as_ref();
        Self::iter().try_for_each(|i| {
            if let Some(striped) = i.strip_prefix(item) {
                let path = path.join(striped);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(path, Self::get(&i).unwrap().data)?;
            }
            Ok(())
        })
    }
}

pub fn setup() {
    #[cfg(target_os = "wasi")]
    std::env::set_current_dir(env!("CARGO_MANIFEST_DIR")).expect("Failed to set current dir");
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

pub fn components_count<P: AsRef<Path>>(p: P) -> usize {
    p.as_ref()
        .components()
        .filter(|it| matches!(it, Component::Normal(_)))
        .count()
}
