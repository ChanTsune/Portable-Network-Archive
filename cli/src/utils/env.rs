use std::path::PathBuf;

pub(crate) fn temp_dir() -> Option<PathBuf> {
    if cfg!(target_os = "wasi") {
        None
    } else {
        Some(std::env::temp_dir())
    }
}
