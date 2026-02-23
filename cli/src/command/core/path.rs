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
    /// When true, keep `Component::CurDir` (`.`) during sanitization (bsdtar-compat).
    preserve_curdir: bool,
}

impl PathnameEditor {
    #[inline]
    pub(crate) const fn new(
        strip_components: Option<usize>,
        transformers: Option<PathTransformers>,
        absolute_paths: bool,
        preserve_curdir: bool,
    ) -> Self {
        Self {
            strip_components,
            transformers,
            absolute_paths,
            preserve_curdir,
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
        if is_effectively_empty_path(&transformed, self.preserve_curdir) {
            return None;
        }
        let stripped = strip_components(&transformed, self.strip_components)?;
        if is_effectively_empty_path(&stripped, self.preserve_curdir) {
            return None;
        }
        let entry_name = EntryName::from_path_lossy_preserve_root(&stripped);
        let sanitized = if self.absolute_paths {
            entry_name
        } else if self.preserve_curdir {
            sanitize_preserve_curdir(entry_name)
        } else {
            entry_name.sanitize()
        };
        if sanitized.as_str().is_empty() {
            return None;
        }
        Some(sanitized)
    }

    /// Edit a hardlink target pathname.
    ///
    /// Returns `None` (skip the entry) when the target becomes empty after
    /// transformation or after stripping.
    pub(crate) fn edit_hardlink(&self, target: &Path) -> Option<EntryReference> {
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
        if is_effectively_empty_path(&transformed, self.preserve_curdir) {
            return None;
        }
        let stripped = strip_components(&transformed, self.strip_components)?;
        if is_effectively_empty_path(&stripped, self.preserve_curdir) {
            return None;
        }
        let entry_reference = EntryReference::from_path_lossy_preserve_root(&stripped);
        let sanitized = if self.absolute_paths {
            entry_reference
        } else if self.preserve_curdir {
            sanitize_preserve_curdir_reference(entry_reference)
        } else {
            entry_reference.sanitize()
        };
        if sanitized.as_str().is_empty() {
            return None;
        }
        Some(sanitized)
    }

    /// Edit a symlink target path.
    ///
    /// Unlike entry names and hardlink targets, symlink targets are never
    /// skipped: the containing entry's name is validated separately via
    /// `edit_entry_name`, and bsdtar does not strip or skip symlink targets.
    pub(crate) fn edit_symlink(&self, target: &Path) -> EntryReference {
        // Note: symlinks do not have strip_components applied (matching bsdtar behavior)
        let transformed: Cow<'_, Path> = if let Some(t) = &self.transformers {
            Cow::Owned(PathBuf::from(t.apply(
                target.to_string_lossy(),
                true,
                false,
            )))
        } else {
            Cow::Borrowed(target)
        };
        let entry_reference = EntryReference::from_path_lossy_preserve_root(&transformed);
        if self.absolute_paths {
            entry_reference
        } else if self.preserve_curdir {
            sanitize_preserve_curdir_reference(entry_reference)
        } else {
            entry_reference.sanitize()
        }
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

/// CLI-side sanitization that keeps `CurDir` (`.`) components.
fn sanitize_preserve_curdir(name: EntryName) -> EntryName {
    let path = Path::new(name.as_str());
    let sanitized: PathBuf = path
        .components()
        .filter(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        .collect();
    EntryName::from_path_lossy_preserve_root(&sanitized)
}

fn sanitize_preserve_curdir_reference(reference: EntryReference) -> EntryReference {
    let path = Path::new(reference.as_str());
    let sanitized: PathBuf = path
        .components()
        .filter(|c| {
            matches!(
                c,
                Component::Normal(_) | Component::CurDir | Component::ParentDir
            )
        })
        .collect();
    EntryReference::from_path_lossy_preserve_root(&sanitized)
}

#[inline]
fn is_effectively_empty_path(path: &Path, preserve_curdir: bool) -> bool {
    if preserve_curdir {
        path.as_os_str().is_empty()
    } else {
        path.components().all(|c| matches!(c, Component::CurDir))
    }
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
        let editor = PathnameEditor::new(None, None, false, false);
        let name = editor.edit_entry_name(Path::new("a/b/c")).unwrap();
        assert_eq!(name.as_str(), "a/b/c");
    }

    #[test]
    fn editor_strip_only() {
        let editor = PathnameEditor::new(Some(1), None, false, false);
        let name = editor.edit_entry_name(Path::new("a/b/c")).unwrap();
        assert_eq!(name.as_str(), "b/c");
    }

    #[test]
    fn editor_strip_insufficient_components() {
        let editor = PathnameEditor::new(Some(5), None, false, false);
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
        let editor = PathnameEditor::new(Some(1), transformers, false, false);

        // With bsdtar order: "old/a/b" -> "new/a/b" -> strip 1 -> "a/b"
        let result = editor.edit_entry_name(Path::new("old/a/b")).unwrap();
        assert_eq!(result.as_str(), "a/b");
    }

    #[test]
    fn editor_symlink_no_strip() {
        // Symlink targets should NOT have strip-components applied (bsdtar behavior)
        let editor = PathnameEditor::new(Some(2), None, false, false);
        let result = editor.edit_symlink(Path::new("a/b/c"));
        assert_eq!(result.as_str(), "a/b/c"); // Not stripped
    }

    #[test]
    fn editor_skips_empty_or_curdir_paths() {
        let editor = PathnameEditor::new(None, None, false, false);
        assert!(editor.edit_entry_name(Path::new("")).is_none());
        assert!(editor.edit_entry_name(Path::new(".")).is_none());
        assert!(editor.edit_entry_name(Path::new("./.")).is_none());
        assert!(editor.edit_hardlink(Path::new("")).is_none());
        assert!(editor.edit_hardlink(Path::new(".")).is_none());
        assert!(editor.edit_hardlink(Path::new("./.")).is_none());
    }

    #[test]
    fn editor_preserve_curdir_keeps_dot_prefix() {
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor.edit_entry_name(Path::new("./a/b")).unwrap();
        assert_eq!(name.as_str(), "./a/b");
    }

    #[test]
    fn editor_preserve_curdir_with_strip_components() {
        // ./target/sub -> strip 1 -> target/sub (the "." component counts as one stripped level)
        let editor = PathnameEditor::new(Some(1), None, false, true);
        let name = editor.edit_entry_name(Path::new("./target/sub")).unwrap();
        assert_eq!(name.as_str(), "target/sub");
    }

    #[test]
    fn editor_preserve_curdir_bare_dot_is_valid() {
        // In bsdtar-compat mode, "." is a valid directory entry
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor.edit_entry_name(Path::new(".")).unwrap();
        assert_eq!(name.as_str(), ".");
    }

    #[test]
    fn editor_preserve_curdir_hardlink() {
        let editor = PathnameEditor::new(None, None, false, true);
        let reference = editor.edit_hardlink(Path::new("./a/b")).unwrap();
        assert_eq!(reference.as_str(), "./a/b");
    }

    #[test]
    fn editor_preserve_curdir_symlink() {
        let editor = PathnameEditor::new(None, None, false, true);
        let reference = editor.edit_symlink(Path::new("./a/b"));
        assert_eq!(reference.as_str(), "./a/b");
    }

    // --- Edge cases: path traversal under preserve_curdir ---

    #[test]
    fn editor_preserve_curdir_strips_parent_dir_from_entry_name() {
        // ParentDir is stripped from entry names for security, even in bsdtar-compat mode
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor.edit_entry_name(Path::new("../a/b")).unwrap();
        assert_eq!(name.as_str(), "a/b");
    }

    #[test]
    fn editor_preserve_curdir_bare_parent_dir_produces_none() {
        // ".." has no Normal or CurDir components; post-sanitization empty check returns None
        let editor = PathnameEditor::new(None, None, false, true);
        assert!(editor.edit_entry_name(Path::new("..")).is_none());
    }

    #[test]
    fn editor_preserve_curdir_only_parent_dirs_produces_none() {
        let editor = PathnameEditor::new(None, None, false, true);
        assert!(editor.edit_entry_name(Path::new("../../..")).is_none());
    }

    #[test]
    fn editor_preserve_curdir_strips_root_from_absolute_path() {
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor.edit_entry_name(Path::new("/etc/passwd")).unwrap();
        assert_eq!(name.as_str(), "etc/passwd");
    }

    #[test]
    fn editor_preserve_curdir_bare_root_produces_none() {
        let editor = PathnameEditor::new(None, None, false, true);
        assert!(editor.edit_entry_name(Path::new("/")).is_none());
    }

    #[test]
    fn editor_preserve_curdir_mixed_curdir_and_parent_dir() {
        // CurDir kept, ParentDir stripped from entry names
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor.edit_entry_name(Path::new("./a/../b")).unwrap();
        assert_eq!(name.as_str(), "./a/b");
    }

    #[test]
    fn editor_preserve_curdir_empty_string_produces_none() {
        let editor = PathnameEditor::new(None, None, false, true);
        assert!(editor.edit_entry_name(Path::new("")).is_none());
        assert!(editor.edit_hardlink(Path::new("")).is_none());
    }

    // --- Edge cases: symlink targets preserve ParentDir (bsdtar stores verbatim) ---

    #[test]
    fn editor_preserve_curdir_symlink_strips_root_from_absolute_target() {
        // Unlike EntryReference::sanitize() which re-inserts RootDir, the CLI-side
        // sanitizer strips it unconditionally — this is the security boundary for
        // absolute symlink targets when absolute_paths=false.
        let editor = PathnameEditor::new(None, None, false, true);
        let reference = editor.edit_symlink(Path::new("/etc/hostname"));
        assert_eq!(reference.as_str(), "etc/hostname");
    }

    #[test]
    fn editor_preserve_curdir_symlink_preserves_parent_dir() {
        // bsdtar preserves .. in symlink targets verbatim
        let editor = PathnameEditor::new(None, None, false, true);
        let reference = editor.edit_symlink(Path::new("../lib"));
        assert_eq!(reference.as_str(), "../lib");
    }

    #[test]
    fn editor_preserve_curdir_symlink_preserves_deep_parent_dir() {
        let editor = PathnameEditor::new(None, None, false, true);
        let reference = editor.edit_symlink(Path::new("../../include/header.h"));
        assert_eq!(reference.as_str(), "../../include/header.h");
    }

    #[test]
    fn editor_preserve_curdir_symlink_mixed_curdir_and_parent_dir() {
        let editor = PathnameEditor::new(None, None, false, true);
        let reference = editor.edit_symlink(Path::new("./a/../b"));
        assert_eq!(reference.as_str(), "./a/../b");
    }

    // --- Edge cases: absolute_paths interaction ---

    #[test]
    fn editor_absolute_paths_takes_priority_over_preserve_curdir() {
        // absolute_paths=true must bypass all sanitization, even when preserve_curdir=true.
        // Guards against if/else-if branch reordering.
        let editor = PathnameEditor::new(None, None, true, true);
        let name = editor.edit_entry_name(Path::new("/etc/passwd")).unwrap();
        assert_eq!(name.as_str(), "/etc/passwd");

        let reference = editor.edit_symlink(Path::new("/etc/hostname"));
        assert_eq!(reference.as_str(), "/etc/hostname");

        let reference = editor.edit_hardlink(Path::new("/etc/hosts")).unwrap();
        assert_eq!(reference.as_str(), "/etc/hosts");
    }

    // --- Edge cases: hardlink targets ---

    #[test]
    fn editor_preserve_curdir_hardlink_preserves_parent_dir() {
        // EntryReference preserves ParentDir (matching EntryReference::sanitize behavior)
        let editor = PathnameEditor::new(None, None, false, true);
        let reference = editor.edit_hardlink(Path::new("../a")).unwrap();
        assert_eq!(reference.as_str(), "../a");
    }

    #[test]
    fn editor_preserve_curdir_hardlink_bare_parent_dir() {
        // ParentDir is kept because is_unsafe_link() guards at extraction time,
        // not at sanitization time — matching bsdtar's separation of concerns.
        let editor = PathnameEditor::new(None, None, false, true);
        let reference = editor.edit_hardlink(Path::new("..")).unwrap();
        assert_eq!(reference.as_str(), "..");
    }

    #[test]
    fn editor_preserve_curdir_hardlink_bare_root_produces_none() {
        let editor = PathnameEditor::new(None, None, false, true);
        assert!(editor.edit_hardlink(Path::new("/")).is_none());
    }

    // --- Edge cases: strip_components interaction ---

    #[test]
    fn editor_preserve_curdir_strip_consumes_all_components() {
        // ./a has 2 components (CurDir + Normal), strip 2 returns None
        let editor = PathnameEditor::new(Some(2), None, false, true);
        assert!(editor.edit_entry_name(Path::new("./a")).is_none());
    }
}
