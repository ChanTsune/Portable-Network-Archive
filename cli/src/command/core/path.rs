use pna::{EntryName, EntryReference};
use std::{
    borrow::Cow,
    path::{Component, Path, PathBuf},
};

use super::PathTransformers;

/// Centralized path transformation editor, similar to libarchive's `edit_pathname`.
///
/// Applies transformations in the following order (matching bsdtar behavior):
/// 1. Apply substitutions/transforms (`-s` or `--transform`)
/// 2. Strip leading path components (`--strip-components`)
#[derive(Clone, Debug)]
pub(crate) struct PathnameEditor {
    strip_components: Option<usize>,
    transformers: Option<PathTransformers>,
    absolute_paths: bool,
}

impl PathnameEditor {
    #[inline]
    pub(crate) const fn new(
        strip_components: Option<usize>,
        transformers: Option<PathTransformers>,
        absolute_paths: bool,
    ) -> Self {
        Self {
            strip_components,
            transformers,
            absolute_paths,
        }
    }

    /// Edit the pathname for a regular archive entry.
    ///
    /// Returns `None` (skip the entry) when the path becomes empty after
    /// transformation or after stripping.
    pub(crate) fn edit_entry_name(&self, path: &Path) -> Option<EntryName> {
        // bsdtar order: substitution first, then strip
        let transformed: Cow<'_, Path> = if let Some(t) = &self.transformers {
            Cow::Owned(PathBuf::from(t.apply(path.to_string_lossy(), false, false)))
        } else {
            Cow::Borrowed(path)
        };
        if is_effectively_empty_path(&transformed) {
            return None;
        }
        let stripped = strip_components(&transformed, self.strip_components)?;
        if is_effectively_empty_path(&stripped) {
            return None;
        }
        let entry_name = EntryName::from_path_lossy_preserve_root(&stripped);
        if self.absolute_paths {
            Some(entry_name)
        } else {
            Some(entry_name.sanitize())
        }
    }

    /// Edit a hardlink target pathname.
    ///
    /// Returns `None` (skip the entry) when the target becomes empty after
    /// transformation or after stripping.
    /// The `bool` indicates whether a leading root component was stripped.
    pub(crate) fn edit_hardlink(&self, target: &Path) -> Option<(EntryReference, bool)> {
        // bsdtar order: substitution first, then strip
        let transformed: Cow<'_, Path> = if let Some(t) = &self.transformers {
            Cow::Owned(PathBuf::from(t.apply(
                target.to_string_lossy(),
                false,
                true,
            )))
        } else {
            Cow::Borrowed(target)
        };
        if is_effectively_empty_path(&transformed) {
            return None;
        }
        let stripped = strip_components(&transformed, self.strip_components)?;
        if is_effectively_empty_path(&stripped) {
            return None;
        }
        let entry_reference = EntryReference::from_path_lossy_preserve_root(&stripped);
        if self.absolute_paths {
            Some((entry_reference, false))
        } else {
            let had_root = stripped.has_root();
            Some((entry_reference.sanitize(), had_root))
        }
    }

    /// Edit a symlink target path.
    ///
    /// Only user-specified substitutions (`-s`) are applied.
    /// Leading `/` and `--strip-components` are NOT applied, matching bsdtar.
    pub(crate) fn edit_symlink(&self, target: &Path) -> EntryReference {
        let transformed: Cow<'_, Path> = if let Some(t) = &self.transformers {
            Cow::Owned(PathBuf::from(t.apply(
                target.to_string_lossy(),
                true,
                false,
            )))
        } else {
            Cow::Borrowed(target)
        };
        EntryReference::from_path_lossy_preserve_root(&transformed)
    }
}

fn strip_components(path: &Path, count: Option<usize>) -> Option<Cow<'_, Path>> {
    let Some(count) = count else {
        return Some(Cow::Borrowed(path));
    };
    if count == 0 {
        return Some(Cow::Borrowed(path));
    }
    let components = path.components();
    if components.clone().count() <= count {
        return None;
    }
    Some(Cow::from(PathBuf::from_iter(components.skip(count))))
}

#[inline]
fn is_effectively_empty_path(path: &Path) -> bool {
    path.components().all(|c| matches!(c, Component::CurDir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_path_none_or_zero() {
        let original = Path::new("a/b");
        match strip_components(original, None).unwrap() {
            Cow::Borrowed(p) => assert_eq!(p, original),
            Cow::Owned(_) => panic!("expected borrowed path when count is None"),
        }
        assert_eq!(
            strip_components(original, Some(0)).unwrap(),
            PathBuf::from("a/b")
        );
    }

    #[test]
    fn strip_path_relative() {
        assert_eq!(
            strip_components(Path::new("a/b/c"), Some(1)).unwrap(),
            PathBuf::from("b/c")
        );
        assert_eq!(
            strip_components(Path::new("a/b/c"), Some(2)).unwrap(),
            PathBuf::from("c")
        );
        assert!(strip_components(Path::new("a/b"), Some(3)).is_none());
    }

    #[test]
    fn strip_path_parent_dir_components() {
        assert_eq!(
            strip_components(Path::new("../a/b"), Some(1)).unwrap(),
            PathBuf::from("a/b")
        );
        assert_eq!(
            strip_components(Path::new("../a/b"), Some(2)).unwrap(),
            PathBuf::from("b")
        );
        assert!(strip_components(Path::new("../a/b"), Some(3)).is_none());
    }

    #[test]
    fn editor_no_transforms() {
        let editor = PathnameEditor::new(None, None, false);
        let name = editor.edit_entry_name(Path::new("a/b/c")).unwrap();
        assert_eq!(name.as_str(), "a/b/c");
    }

    #[test]
    fn editor_strip_only() {
        let editor = PathnameEditor::new(Some(1), None, false);
        let name = editor.edit_entry_name(Path::new("a/b/c")).unwrap();
        assert_eq!(name.as_str(), "b/c");
    }

    #[test]
    fn editor_strip_insufficient_components() {
        let editor = PathnameEditor::new(Some(5), None, false);
        assert!(editor.edit_entry_name(Path::new("a/b")).is_none());
        assert!(editor.edit_hardlink(Path::new("a/b")).is_none());
    }

    #[test]
    fn editor_bsdtar_order_transform_then_strip() {
        // bsdtar applies substitutions BEFORE strip-components.
        // Example: path "old/a/b" with transform /old/new/ and strip=1
        // bsdtar order: "old/a/b" -> transform -> "new/a/b" -> strip 1 -> "a/b"
        // (wrong order would be: "old/a/b" -> strip 1 -> "a/b" -> transform -> "a/b")
        use super::super::{PathTransformers, re::bsd::SubstitutionRules};

        // Format: /pattern/replacement/flags - first char is the delimiter
        let rules = SubstitutionRules::new(vec!["/old/new/".parse().unwrap()]);
        let transformers = Some(PathTransformers::BsdSubstitutions(rules));
        let editor = PathnameEditor::new(Some(1), transformers, false);

        // With bsdtar order: "old/a/b" -> "new/a/b" -> strip 1 -> "a/b"
        let result = editor.edit_entry_name(Path::new("old/a/b")).unwrap();
        assert_eq!(result.as_str(), "a/b");
    }

    #[test]
    fn editor_symlink_no_strip() {
        // Symlink targets should NOT have strip-components applied (bsdtar behavior)
        let editor = PathnameEditor::new(Some(2), None, false);
        let result = editor.edit_symlink(Path::new("a/b/c"));
        assert_eq!(result.as_str(), "a/b/c"); // Not stripped
    }

    #[test]
    fn editor_skips_empty_or_curdir_paths() {
        let editor = PathnameEditor::new(None, None, false);
        assert!(editor.edit_entry_name(Path::new("")).is_none());
        assert!(editor.edit_entry_name(Path::new(".")).is_none());
        assert!(editor.edit_entry_name(Path::new("./.")).is_none());
        assert!(editor.edit_hardlink(Path::new("")).is_none());
        assert!(editor.edit_hardlink(Path::new(".")).is_none());
        assert!(editor.edit_hardlink(Path::new("./.")).is_none());
    }
}
