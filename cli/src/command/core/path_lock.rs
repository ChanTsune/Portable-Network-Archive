use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// Maintains per-path mutexes so that concurrent archive operations update a given path in order.
///
/// The registry stores a lazily-created `Arc<Mutex<()>>` for each canonical path. Callers clone the
/// returned `Arc` and lock it before touching the filesystem, ensuring that all work targeting the
/// same output path is serialized even when the caller itself is running inside a parallel context.
#[derive(Debug, Default)]
pub(crate) struct PathLocks {
    inner: Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>,
}

impl PathLocks {
    /// Returns the mutex guarding `path`, creating it on demand.
    ///
    /// # Panics
    ///
    /// Panics if the internal registry lock is poisoned, which only happens if another thread
    /// holding it panicked.
    pub(crate) fn get(&self, path: &Path) -> Arc<Mutex<()>> {
        let mut map = self
            .inner
            .lock()
            .expect("path lock registry mutex poisoned");
        Arc::clone(
            map.entry(path.to_path_buf())
                .or_insert_with(|| Arc::new(Mutex::new(()))),
        )
    }
}
