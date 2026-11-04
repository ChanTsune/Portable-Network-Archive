use path_slash::PathExt as _;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// Two directory trees must be structurally and content-identical.
/// Panics with a descriptive message on any difference.
/// Does NOT compare metadata (permissions, timestamps, xattrs).
pub fn assert_dirs_equal(expected: impl AsRef<Path>, actual: impl AsRef<Path>) {
    let expected = expected.as_ref();
    let actual = actual.as_ref();
    let expected_entries = collect_entries(expected);
    let actual_entries = collect_entries(actual);

    // Pass 1: Check expected entries exist in actual and compare
    for (rel, expected_kind) in &expected_entries {
        let actual_kind = actual_entries.get(rel).unwrap_or_else(|| {
            panic!(
                "missing in actual: {} (expected in {})",
                rel.display(),
                actual.display()
            )
        });
        let expected_path = expected.join(rel);
        let actual_path = actual.join(rel);
        assert_eq!(
            expected_kind,
            actual_kind,
            "type mismatch at {}: expected {:?}, got {:?}",
            rel.display(),
            expected_kind,
            actual_kind
        );
        match expected_kind {
            EntryKind::File => {
                let expected_bytes = fs::read(&expected_path).unwrap_or_else(|e| {
                    panic!("failed to read {}: {}", expected_path.display(), e)
                });
                let actual_bytes = fs::read(&actual_path)
                    .unwrap_or_else(|e| panic!("failed to read {}: {}", actual_path.display(), e));
                assert!(
                    expected_bytes == actual_bytes,
                    "content differs at {}:\n  expected: {} ({} bytes)\n  actual:   {} ({} bytes)",
                    rel.display(),
                    expected_path.display(),
                    expected_bytes.len(),
                    actual_path.display(),
                    actual_bytes.len()
                );
            }
            EntryKind::Symlink => {
                let expected_target = fs::read_link(&expected_path).unwrap_or_else(|e| {
                    panic!("failed to read symlink {}: {}", expected_path.display(), e)
                });
                let actual_target = fs::read_link(&actual_path).unwrap_or_else(|e| {
                    panic!("failed to read symlink {}: {}", actual_path.display(), e)
                });
                assert_eq!(
                    expected_target,
                    actual_target,
                    "symlink target differs at {}",
                    rel.display()
                );
            }
            EntryKind::Dir => {} // existence already verified
        }
    }

    // Pass 2: Check for unexpected entries in actual
    for rel in actual_entries.keys() {
        assert!(
            expected_entries.contains_key(rel),
            "unexpected in actual: {} (not in {})",
            rel.display(),
            expected.display()
        );
    }
}

#[derive(Debug, PartialEq, Eq)]
enum EntryKind {
    File,
    Dir,
    Symlink,
}

fn collect_entries(dir: &Path) -> BTreeMap<PathBuf, EntryKind> {
    let mut entries = BTreeMap::new();
    for entry in walkdir::WalkDir::new(dir).follow_links(false).min_depth(1) {
        let entry = entry.unwrap_or_else(|e| panic!("failed to walk {}: {}", dir.display(), e));
        let ft = entry.file_type();
        let rel = entry.path().strip_prefix(dir).unwrap();
        let rel = PathBuf::from(
            rel.to_slash()
                .unwrap_or_else(|| panic!("non-UTF-8 path: {}", rel.display()))
                .into_owned(),
        );
        let kind = if ft.is_symlink() {
            EntryKind::Symlink
        } else if ft.is_file() {
            EntryKind::File
        } else if ft.is_dir() {
            EntryKind::Dir
        } else {
            panic!("unsupported file type at {}", entry.path().display());
        };
        let prev = entries.insert(rel.clone(), kind);
        assert!(
            prev.is_none(),
            "duplicate entry after normalization: {}",
            rel.display()
        );
    }
    entries
}
