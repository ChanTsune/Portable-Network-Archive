use std::collections::HashSet;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub fn diff<P1: AsRef<Path>, P2: AsRef<Path>>(dir1: P1, dir2: P2) -> io::Result<()> {
    let differences = diff_dirs(dir1.as_ref(), dir2.as_ref())?;
    if differences.is_empty() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{:?}", differences),
        ))
    }
}

fn diff_dirs(dir1: &Path, dir2: &Path) -> io::Result<Vec<String>> {
    let mut differences = Vec::new();
    let entries1 = read_dir_recursively(dir1)?;
    let entries2 = read_dir_recursively(dir2)?;

    let entries1_set: HashSet<_> = entries1.keys().collect();
    let entries2_set: HashSet<_> = entries2.keys().collect();

    // Files only in dir1
    for entry in entries1_set.difference(&entries2_set) {
        differences.push(format!("Only in {}: {}", dir1.display(), entry.display()));
    }

    // Files only in dir2
    for entry in entries2_set.difference(&entries1_set) {
        differences.push(format!("Only in {}: {}", dir2.display(), entry.display()));
    }

    // Compare common files
    for entry in entries1_set.intersection(&entries2_set) {
        let path1 = dir1.join(entries1_set.get(entry).unwrap());
        let path2 = dir2.join(entries2_set.get(entry).unwrap());
        if path1.is_file() && path2.is_file() {
            if !compare_files(&path1, &path2)? {
                differences.push(format!(
                    "Files differ: {} and {}",
                    path1.display(),
                    path2.display()
                ));
            }
        } else if (path1.is_file() && path2.is_dir()) || (path1.is_dir() && path2.is_file()) {
            differences.push(format!(
                "File type differ: {} and {}",
                path1.display(),
                path2.display()
            ));
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
    let mut f1 = fs::File::open(file1)?;
    let mut f2 = fs::File::open(file2)?;

    let mut buffer1 = Vec::new();
    let mut buffer2 = Vec::new();

    f1.read_to_end(&mut buffer1)?;
    f2.read_to_end(&mut buffer2)?;

    Ok(buffer1 == buffer2)
}
