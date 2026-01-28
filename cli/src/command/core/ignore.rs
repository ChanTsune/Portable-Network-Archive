use ignore::gitignore::Gitignore;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// Gitignore-style exclusion rules.
#[derive(Default)]
pub(crate) struct Ignore {
    by_dir: HashMap<PathBuf, Gitignore>,
}

impl Ignore {
    #[inline]
    pub(crate) fn is_ignore(&self, path: impl AsRef<Path>, is_dir: bool) -> bool {
        let path = path.as_ref();
        // Start from the directory containing the path (or the path itself if it is a dir),
        // walk up to root, and apply the nearest .gitignore last (closest wins).
        // Determine the first directory to check for a .gitignore
        let mut cur_dir_opt = if is_dir { Some(path) } else { path.parent() };

        while let Some(dir) = cur_dir_opt {
            if let Some(gi) = self.by_dir.get(dir) {
                let rel = path.strip_prefix(dir).unwrap_or(path);
                let m = gi.matched(rel, is_dir);
                if m.is_ignore() {
                    return true;
                }
                if m.is_whitelist() {
                    return false;
                }
            }
            cur_dir_opt = dir.parent();
        }
        false
    }

    #[inline]
    pub(crate) fn add_path(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        debug_assert!(path.is_dir());
        let gitignore_path = path.join(".gitignore");
        if gitignore_path.exists() {
            let (ig, err) = Gitignore::new(&gitignore_path);
            if let Some(e) = err {
                log::warn!(
                    "Failed to fully parse .gitignore at '{}': {}",
                    gitignore_path.display(),
                    e
                );
            }
            self.by_dir.insert(path.to_path_buf(), ig);
        }
    }
}
