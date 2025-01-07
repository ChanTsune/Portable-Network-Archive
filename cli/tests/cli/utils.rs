pub mod diff;

use std::{
    fs, io,
    path::{Component, Path},
};

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
