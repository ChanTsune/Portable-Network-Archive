use std::collections::HashSet;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(thiserror::Error, Debug, Eq, PartialEq, Hash, Clone)]
pub enum DiffError {
    #[error("`{1}` is only in `{0}`")]
    OnlyIn(String, String),
    #[error("files differ: `{0}` and `{1}` ")]
    DifferentContent(String, String),
    #[error("file type differ `{0}` and `{1}` ")]
    DifferentType(String, String),
}

impl DiffError {
    #[inline]
    pub fn only_in(dir: impl ToString, item: impl ToString) -> Self {
        Self::OnlyIn(dir.to_string(), item.to_string())
    }
    #[inline]
    pub fn different_content(dir: impl ToString, item: impl ToString) -> Self {
        Self::DifferentContent(dir.to_string(), item.to_string())
    }
    #[inline]
    pub fn different_type(dir: impl ToString, item: impl ToString) -> Self {
        Self::DifferentType(dir.to_string(), item.to_string())
    }
}

pub fn diff<P1: AsRef<Path>, P2: AsRef<Path>>(
    dir1: P1,
    dir2: P2,
) -> io::Result<HashSet<DiffError>> {
    diff_dirs(dir1.as_ref(), dir2.as_ref())
}

fn diff_dirs(dir1: &Path, dir2: &Path) -> io::Result<HashSet<DiffError>> {
    let mut differences = HashSet::new();
    let entries1 = read_dir_recursively(dir1)?;
    let entries2 = read_dir_recursively(dir2)?;

    let entries1_set: HashSet<_> = entries1.keys().collect();
    let entries2_set: HashSet<_> = entries2.keys().collect();

    // Files only in dir1
    for entry in entries1_set.difference(&entries2_set) {
        differences.insert(DiffError::only_in(dir1.display(), entry.display()));
    }

    // Files only in dir2
    for entry in entries2_set.difference(&entries1_set) {
        differences.insert(DiffError::only_in(dir2.display(), entry.display()));
    }

    // Compare common files
    for entry in entries1_set.intersection(&entries2_set) {
        let path1 = dir1.join(entries1_set.get(entry).unwrap());
        let path2 = dir2.join(entries2_set.get(entry).unwrap());
        if path1.is_file() && path2.is_file() {
            if !compare_files(&path1, &path2)? {
                differences.insert(DiffError::different_content(
                    path1.display(),
                    path2.display(),
                ));
            }
        } else if (path1.is_file() && path2.is_dir()) || (path1.is_dir() && path2.is_file()) {
            differences.insert(DiffError::different_type(path1.display(), path2.display()));
        }
    }

    Ok(differences)
}

fn read_dir_recursively(dir: &Path) -> io::Result<std::collections::HashMap<PathBuf, PathBuf>> {
    let mut entries = std::collections::HashMap::new();
    for entry in walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.is_file() || path.is_dir() {
            let relative_path = path.strip_prefix(dir).unwrap();
            entries.insert(relative_path.to_path_buf(), path.to_path_buf());
        }
    }
    Ok(entries)
}

fn compare_files(file1: &Path, file2: &Path) -> io::Result<bool> {
    let buffer1 = fs::read(file1)?;
    let buffer2 = fs::read(file2)?;
    Ok(buffer1 == buffer2)
}
