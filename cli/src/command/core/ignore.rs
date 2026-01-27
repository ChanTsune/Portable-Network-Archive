use ignore::gitignore::Gitignore;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// Gitignore-style exclusion rules.
pub(crate) struct Ignore {
    // Map of directory path -> compiled .gitignore matcher for that directory
    by_dir: HashMap<PathBuf, Gitignore>,
}

impl Ignore {
    #[inline]
    pub(crate) fn empty() -> Self {
        Self {
            by_dir: HashMap::new(),
        }
    }

    #[inline]
    pub(crate) fn is_ignore(&self, path: impl AsRef<Path>, is_dir: bool) -> bool {
        let path = path.as_ref();
        // Start from the directory containing the path (or the path itself if it is a dir),
        // walk up to root, and apply the nearest .gitignore last (closest wins).
        // Determine the first directory to check for a .gitignore
        let mut cur_dir_opt = if is_dir { Some(path) } else { path.parent() };

        while let Some(dir) = cur_dir_opt {
            if let Some(gi) = self.by_dir.get(dir) {
                // Match relative to the directory of the .gitignore
                let rel = path.strip_prefix(dir).unwrap_or(path);
                let m = gi.matched(rel, is_dir);
                // If this matcher provides a decision, return immediately; closest wins
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
            let (ig, _) = Gitignore::new(&gitignore_path);
            // Key by the directory that owns this .gitignore
            self.by_dir.insert(path.to_path_buf(), ig);
        }
    }
}
