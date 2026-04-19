use pna::{EntryName, EntryReference};
use std::{
    borrow::Cow,
    path::{Component, Path, PathBuf},
};
use typed_path::{Utf8WindowsComponent, Utf8WindowsPath};

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
        let stripped = self.transform_and_strip(path, false, false)?;
        let rewritten = self.rewrite_path_for_extraction(&stripped);
        let entry_name = EntryName::from_utf8_preserve_root(rewritten.path.as_ref());
        let sanitized = if self.absolute_paths {
            entry_name
        } else if self.preserve_curdir {
            sanitize_preserve_curdir(entry_name)
        } else {
            entry_name.sanitize()
        };
        self.check_nodotdot(sanitized.as_str(), "skipping")?;
        Some(sanitized)
    }

    /// Edit a hardlink target pathname.
    ///
    /// Returns `None` (skip the entry) when the target becomes empty after
    /// transformation or after stripping.
    /// The `bool` indicates whether a leading root component was stripped.
    pub(crate) fn edit_hardlink(&self, target: &Path) -> Option<(EntryReference, bool)> {
        let stripped = self.transform_and_strip(target, false, true)?;
        let rewritten = self.rewrite_path_for_extraction(&stripped);
        let entry_reference = EntryReference::from_utf8_preserve_root(rewritten.path.as_ref());
        let had_root = rewritten.had_root;
        let sanitized = if self.absolute_paths {
            entry_reference
        } else if self.preserve_curdir {
            sanitize_preserve_curdir_reference(entry_reference)
        } else {
            entry_reference.sanitize()
        };
        self.check_nodotdot(sanitized.as_str(), "skipping hardlink")?;
        Some((sanitized, had_root))
    }

    /// Apply user-specified substitutions to a link target while preserving
    /// absolute path components and skipping `--strip-components`, matching
    /// bsdtar symlink semantics. Shared between [`edit_symlink`](Self::edit_symlink)
    /// and [`edit_junction`](Self::edit_junction).
    fn transform_link_target_preserving_root(&self, target: &Path) -> EntryReference {
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

    /// Edit a symlink target path.
    ///
    /// Only user-specified substitutions (`-s`) are applied.
    /// Leading `/` and `--strip-components` are NOT applied, matching bsdtar.
    pub(crate) fn edit_symlink(&self, target: &Path) -> EntryReference {
        self.transform_link_target_preserving_root(target)
    }

    /// Edit a Windows-junction target path.
    ///
    /// Only user-specified substitutions (`-s`) are applied.
    /// Leading `/` and `--strip-components` are NOT applied, matching bsdtar
    /// symlink semantics.
    ///
    /// Semantically identical to [`edit_symlink`](Self::edit_symlink) for the
    /// moment. A separate public method is introduced so that any future
    /// divergence between symlink-target and junction-target handling can be
    /// added without touching every call site.
    pub(crate) fn edit_junction(&self, target: &Path) -> EntryReference {
        self.transform_link_target_preserving_root(target)
    }

    /// Apply substitution transforms and strip leading components.
    ///
    /// Returns `None` when the path becomes empty after transformation or stripping.
    fn transform_and_strip(
        &self,
        path: &Path,
        is_symlink: bool,
        is_hardlink: bool,
    ) -> Option<PathBuf> {
        let transformed: Cow<'_, Path> = if let Some(t) = &self.transformers {
            Cow::Owned(PathBuf::from(t.apply(
                path.to_string_lossy(),
                is_symlink,
                is_hardlink,
            )))
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
        Some(stripped.into_owned())
    }

    /// Rewrite the path by stripping Windows-style absolute prefixes (drive letters,
    /// UNC/API prefixes) and leading root separators, matching bsdtar's
    /// `strip_absolute_path()` behavior. When `absolute_paths` is true, the path
    /// is returned unchanged.
    #[inline]
    fn rewrite_path_for_extraction<'a>(&self, path: &'a Path) -> RewrittenPath<'a> {
        let raw = path.to_string_lossy();
        if self.absolute_paths {
            RewrittenPath {
                path: raw,
                had_root: false,
            }
        } else {
            match raw {
                Cow::Borrowed(s) => {
                    let (stripped, had_root) = strip_absolute_path_bsdtar(s);
                    RewrittenPath {
                        path: Cow::Borrowed(stripped),
                        had_root,
                    }
                }
                Cow::Owned(s) => {
                    let (stripped, had_root) = strip_absolute_path_bsdtar(&s);
                    let offset = s.len() - stripped.len();
                    let mut owned = s;
                    if offset > 0 {
                        owned.drain(..offset);
                    }
                    RewrittenPath {
                        path: Cow::Owned(owned),
                        had_root,
                    }
                }
            }
        }
    }

    /// bsdtar-compat: SECURE_NODOTDOT -- reject paths containing `..`.
    ///
    /// Returns `None` when the sanitized path is empty or contains `..` in
    /// preserve_curdir mode without absolute_paths.
    fn check_nodotdot(&self, sanitized: &str, context: &str) -> Option<()> {
        if sanitized.is_empty() {
            return None;
        }
        if self.preserve_curdir && !self.absolute_paths && has_parent_dir_component(sanitized) {
            log::warn!("Path contains '..', {}: {}", context, sanitized);
            return None;
        }
        Some(())
    }
}

/// `had_root` indicates whether any absolute prefix (root separator, drive
/// letter, or Windows API prefix) was consumed during rewriting.
struct RewrittenPath<'a> {
    path: Cow<'a, str>,
    had_root: bool,
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

/// CLI-side sanitization that retains `CurDir` (`.`) and `ParentDir` (`..`)
/// while stripping `RootDir` and `Prefix` components via [`std::path::Path::components`].
///
/// Absolute path prefix stripping (Windows drive letters, UNC prefixes,
/// leading `/../` sequences) is handled upstream by [`strip_absolute_path_bsdtar`];
/// this function handles residual host-path normalization. On Unix,
/// backslash-containing segments are treated as literal `Normal` components by
/// the standard library, which preserves them as-is.
fn sanitize_preserve_curdir_str(s: &str) -> String {
    let path = Path::new(s);
    join_components_forward_slash(path.components().filter(|c| {
        matches!(
            c,
            Component::Normal(_) | Component::CurDir | Component::ParentDir
        )
    }))
}

fn sanitize_preserve_curdir(name: EntryName) -> EntryName {
    let sanitized = sanitize_preserve_curdir_str(name.as_str());
    if sanitized.is_empty() {
        return EntryName::from_utf8_preserve_root(".");
    }
    EntryName::from_utf8_preserve_root(&sanitized)
}

fn sanitize_preserve_curdir_reference(reference: EntryReference) -> EntryReference {
    EntryReference::from_utf8_preserve_root(&sanitize_preserve_curdir_str(reference.as_str()))
}

/// Returns `true` if the path is unsafe as a link reference.
///
/// A link is unsafe if it contains an absolute path component (root separator,
/// drive letter, or Windows API prefix) or a parent directory (`..`) component
/// under either host or Windows path semantics.
pub(crate) fn is_unsafe_link_path(s: &str) -> bool {
    let (rewritten, had_root) = strip_absolute_path_bsdtar(s);
    had_root || has_parent_dir_component(rewritten)
}

/// Returns `true` if the path contains a `..` (parent directory) component
/// under either host path semantics or Windows path semantics.
///
/// The dual check ensures that Windows-style `..` preceded by backslash
/// separators (e.g., `..\\file`) is detected even on non-Windows hosts where
/// `std::path::Path` treats backslashes as literal characters.
fn has_parent_dir_component(s: &str) -> bool {
    Path::new(s)
        .components()
        .any(|c| matches!(c, Component::ParentDir))
        || Utf8WindowsPath::new(s)
            .components()
            .any(|c| matches!(c, Utf8WindowsComponent::ParentDir))
}

/// bsdtar-compatible stripping of absolute path prefixes.
///
/// Strips Windows API prefixes (`\\?\`, `\\.\`), UNC prefixes (`\\?\UNC\`),
/// drive letters (`C:`), and leading separators (including consuming `/../`
/// and `/./` sequences that follow a root).
///
/// Returns the remaining path after stripping and a flag indicating whether
/// any prefix or root separator was consumed.
///
/// # Security invariant
///
/// This function may leave `..` components in the output (e.g., from `D:..`).
/// Callers **must** check the result with [`has_parent_dir_component`] to
/// detect path traversal, or use [`is_unsafe_link_path`] which combines both.
fn strip_absolute_path_bsdtar(path: &str) -> (&str, bool) {
    let mut rest = path;
    let mut had_root = false;

    if matches_windows_api_prefix(rest) {
        if matches_unc_api_prefix(rest) {
            rest = &rest[8..];
        } else {
            rest = &rest[4..];
        }
        had_root = true;
    }

    loop {
        let mut advanced = false;

        if is_drive_letter_prefix(rest) {
            rest = &rest[2..];
            had_root = true;
            advanced = true;
        }

        while let Some(sep) = rest.chars().next() {
            if !is_path_separator(sep) {
                break;
            }

            let bytes = rest.as_bytes();
            if bytes.len() >= 4
                && bytes[1] == b'.'
                && bytes[2] == b'.'
                && is_path_separator(bytes[3] as char)
            {
                rest = &rest[3..];
            } else if bytes.len() >= 3 && bytes[1] == b'.' && is_path_separator(bytes[2] as char) {
                rest = &rest[2..];
            } else {
                rest = &rest[1..];
            }
            had_root = true;
            advanced = true;
        }

        if !advanced {
            break;
        }
    }

    (rest, had_root)
}

#[inline]
fn matches_windows_api_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 4
        && is_path_separator(bytes[0] as char)
        && is_path_separator(bytes[1] as char)
        && matches!(bytes[2], b'.' | b'?')
        && is_path_separator(bytes[3] as char)
}

#[inline]
fn matches_unc_api_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 8
        && matches_windows_api_prefix(path)
        && bytes[2] == b'?'
        && matches!(bytes[4], b'U' | b'u')
        && matches!(bytes[5], b'N' | b'n')
        && matches!(bytes[6], b'C' | b'c')
        && is_path_separator(bytes[7] as char)
}

#[inline]
fn is_drive_letter_prefix(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

#[inline]
fn is_path_separator(c: char) -> bool {
    matches!(c, '/' | '\\')
}

/// Join path components with `/` separator to produce platform-independent archive paths.
fn join_components_forward_slash<'a>(mut iter: impl Iterator<Item = Component<'a>>) -> String {
    let Some(first) = iter.next() else {
        return String::new();
    };
    let mut result = first.as_os_str().to_string_lossy().into_owned();
    for component in iter {
        result.push('/');
        result.push_str(&component.as_os_str().to_string_lossy());
    }
    result
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
        let (reference, had_root) = editor.edit_hardlink(Path::new("./a/b")).unwrap();
        assert_eq!(reference.as_str(), "./a/b");
        assert!(!had_root);
    }

    #[test]
    fn editor_preserve_curdir_symlink() {
        let editor = PathnameEditor::new(None, None, false, true);
        let reference = editor.edit_symlink(Path::new("./a/b"));
        assert_eq!(reference.as_str(), "./a/b");
    }

    // --- Edge cases: path traversal under preserve_curdir ---

    #[test]
    fn editor_preserve_curdir_rejects_entry_with_parent_dir() {
        // bsdtar-compat: SECURE_NODOTDOT rejects entry names containing ".."
        let editor = PathnameEditor::new(None, None, false, true);
        assert!(editor.edit_entry_name(Path::new("../a/b")).is_none());
    }

    #[test]
    fn editor_preserve_curdir_bare_parent_dir_produces_none() {
        // bsdtar-compat: SECURE_NODOTDOT rejects bare ".."
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
    fn editor_preserve_curdir_bare_root_becomes_dot() {
        // bsdtar converts "/" to "." via cleanup_pathname_fsobj
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor.edit_entry_name(Path::new("/")).unwrap();
        assert_eq!(name.as_str(), ".");
    }

    #[test]
    fn editor_preserve_curdir_mixed_curdir_and_parent_dir() {
        // bsdtar-compat: SECURE_NODOTDOT rejects any path containing ".."
        let editor = PathnameEditor::new(None, None, false, true);
        assert!(editor.edit_entry_name(Path::new("./a/../b")).is_none());
    }

    #[test]
    fn editor_preserve_curdir_empty_string_produces_none() {
        let editor = PathnameEditor::new(None, None, false, true);
        assert!(editor.edit_entry_name(Path::new("")).is_none());
        assert!(editor.edit_hardlink(Path::new("")).is_none());
    }

    // --- Edge cases: symlink targets preserve ParentDir (bsdtar stores verbatim) ---

    #[test]
    fn editor_preserve_curdir_symlink_preserves_absolute_target() {
        // bsdtar passes symlink targets verbatim — no cleanup_pathname_fsobj.
        // is_unsafe_link() guards at extraction time when allow_unsafe_links=false.
        let editor = PathnameEditor::new(None, None, false, true);
        let reference = editor.edit_symlink(Path::new("/etc/hostname"));
        assert_eq!(reference.as_str(), "/etc/hostname");
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

    #[test]
    fn editor_preserve_curdir_symlink_bare_root() {
        // bsdtar passes symlink targets verbatim — "/" is preserved as-is.
        let editor = PathnameEditor::new(None, None, false, true);
        let reference = editor.edit_symlink(Path::new("/"));
        assert_eq!(reference.as_str(), "/");
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

        let (reference, had_root) = editor.edit_hardlink(Path::new("/etc/hosts")).unwrap();
        assert_eq!(reference.as_str(), "/etc/hosts");
        assert!(!had_root);
    }

    #[test]
    fn editor_preserve_curdir_absolute_paths_allows_parent_dir() {
        // -P disables SECURE_NODOTDOT: ".." in entry names is allowed
        let editor = PathnameEditor::new(None, None, true, true);
        let name = editor.edit_entry_name(Path::new("a/../b")).unwrap();
        assert_eq!(name.as_str(), "a/../b");
    }

    // --- Edge cases: hardlink targets ---

    #[test]
    fn editor_preserve_curdir_hardlink_rejects_parent_dir() {
        // bsdtar-compat: SECURE_NODOTDOT applies to hardlink targets
        let editor = PathnameEditor::new(None, None, false, true);
        assert!(editor.edit_hardlink(Path::new("../a")).is_none());
    }

    #[test]
    fn editor_preserve_curdir_hardlink_bare_parent_dir_produces_none() {
        // bsdtar-compat: SECURE_NODOTDOT rejects bare ".."
        let editor = PathnameEditor::new(None, None, false, true);
        assert!(editor.edit_hardlink(Path::new("..")).is_none());
    }

    #[test]
    fn editor_preserve_curdir_hardlink_absolute_paths_allows_parent_dir() {
        // -P disables SECURE_NODOTDOT for hardlink targets
        let editor = PathnameEditor::new(None, None, true, true);
        let (reference, had_root) = editor.edit_hardlink(Path::new("a/../b")).unwrap();
        assert_eq!(reference.as_str(), "a/../b");
        assert!(!had_root);
    }

    #[test]
    fn editor_preserve_curdir_hardlink_bare_root_produces_none() {
        let editor = PathnameEditor::new(None, None, false, true);
        assert!(editor.edit_hardlink(Path::new("/")).is_none());
    }

    // --- Edge cases: leading /../ stripping (bsdtar strip_absolute_path compat) ---

    #[test]
    fn editor_preserve_curdir_strips_leading_dotdot_after_root() {
        // bsdtar's strip_absolute_path consumes leading /../ as part of absolute prefix
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor.edit_entry_name(Path::new("/../a")).unwrap();
        assert_eq!(name.as_str(), "a");
    }

    #[test]
    fn editor_preserve_curdir_strips_multiple_leading_dotdot_after_root() {
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor.edit_entry_name(Path::new("/../../a")).unwrap();
        assert_eq!(name.as_str(), "a");
    }

    #[test]
    fn editor_preserve_curdir_hardlink_strips_leading_dotdot_after_root() {
        let editor = PathnameEditor::new(None, None, false, true);
        let (reference, had_root) = editor.edit_hardlink(Path::new("/../a")).unwrap();
        assert_eq!(reference.as_str(), "a");
        assert!(had_root);
    }

    // --- Edge cases: strip_components interaction ---

    #[test]
    fn editor_preserve_curdir_strip_consumes_all_components() {
        // ./a has 2 components (CurDir + Normal), strip 2 returns None
        let editor = PathnameEditor::new(Some(2), None, false, true);
        assert!(editor.edit_entry_name(Path::new("./a")).is_none());
    }

    // --- Windows path prefix stripping (bsdtar test_windows compat) ---
    //
    // bsdtar's strip_absolute_path() strips Windows API prefixes, drive letters,
    // and leading separators. On Windows, Rust's Path::components() correctly
    // parses these as Prefix/RootDir components. The `rewrite_path_for_extraction`
    // method calls `strip_absolute_path_bsdtar` to handle this at the string
    // level before host-path normalization. The cross-platform tests below this
    // section cover the same scenarios without #[cfg(windows)].
    //
    // These 8 types correspond to bsdtar's test_windows.c mkfullpath() types.

    #[cfg(windows)]
    #[test]
    fn editor_windows_type0_forward_slash_absolute() {
        // Type 0: /path/to/file — leading / stripped
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor
            .edit_entry_name(Path::new("/msys64/tmp/file"))
            .unwrap();
        assert_eq!(name.as_str(), "msys64/tmp/file");
    }

    #[cfg(windows)]
    #[test]
    fn editor_windows_type1_backslash_absolute() {
        // Type 1: \path\to\file — leading \ stripped
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor
            .edit_entry_name(Path::new("\\msys64\\tmp\\file"))
            .unwrap();
        assert_eq!(name.as_str(), "msys64/tmp/file");
    }

    #[cfg(windows)]
    #[test]
    fn editor_windows_type2_drive_forward_slash() {
        // Type 2: C:/path/to/file — C: and / stripped
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor
            .edit_entry_name(Path::new("C:/msys64/tmp/file"))
            .unwrap();
        assert_eq!(name.as_str(), "msys64/tmp/file");
    }

    #[cfg(windows)]
    #[test]
    fn editor_windows_type3_drive_backslash() {
        // Type 3: C:\path\to\file — C: and \ stripped
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor
            .edit_entry_name(Path::new("C:\\msys64\\tmp\\file"))
            .unwrap();
        assert_eq!(name.as_str(), "msys64/tmp/file");
    }

    #[cfg(windows)]
    #[test]
    fn editor_windows_type4_device_forward_slash() {
        // Type 4: //./C:/path/to/file — //./ and C: and / stripped
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor
            .edit_entry_name(Path::new("//./C:/msys64/tmp/file"))
            .unwrap();
        assert_eq!(name.as_str(), "msys64/tmp/file");
    }

    #[cfg(windows)]
    #[test]
    fn editor_windows_type5_device_backslash() {
        // Type 5: \\.\C:\path\to\file — \\.\ and C: and \ stripped
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor
            .edit_entry_name(Path::new("\\\\.\\C:\\msys64\\tmp\\file"))
            .unwrap();
        assert_eq!(name.as_str(), "msys64/tmp/file");
    }

    #[cfg(windows)]
    #[test]
    fn editor_windows_type6_verbatim_forward_slash() {
        // Type 6: //?/C:/path/to/file — //?/ and C: and / stripped
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor
            .edit_entry_name(Path::new("//?/C:/msys64/tmp/file"))
            .unwrap();
        assert_eq!(name.as_str(), "msys64/tmp/file");
    }

    #[cfg(windows)]
    #[test]
    fn editor_windows_type7_verbatim_backslash() {
        // Type 7: \\?\C:\path\to\file — \\?\ and C: and \ stripped
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor
            .edit_entry_name(Path::new("\\\\?\\C:\\msys64\\tmp\\file"))
            .unwrap();
        assert_eq!(name.as_str(), "msys64/tmp/file");
    }

    #[cfg(windows)]
    #[test]
    fn editor_windows_backslash_to_forward_slash_conversion() {
        // bsdtar converts \ to / in entry names; pna's join_components_forward_slash
        // achieves the same by reconstructing paths with / separators.
        let editor = PathnameEditor::new(None, None, false, true);
        let name = editor.edit_entry_name(Path::new("C:\\a\\b\\c")).unwrap();
        assert_eq!(name.as_str(), "a/b/c");
    }

    #[test]
    fn strip_absolute_path_bsdtar_handles_windows_prefixes() {
        let (path, had_root) = strip_absolute_path_bsdtar("c:/file04");
        assert_eq!("file04", path);
        assert!(had_root);

        let (path, had_root) = strip_absolute_path_bsdtar("//?/UNC/server/share/file15");
        assert_eq!("server/share/file15", path);
        assert!(had_root);

        let (path, had_root) = strip_absolute_path_bsdtar("\\\\?\\UNC\\server\\share\\file35");
        assert_eq!("server\\share\\file35", path);
        assert!(had_root);

        let (path, had_root) = strip_absolute_path_bsdtar("\\/?/uNc/server\\share\\file52");
        assert_eq!("server\\share\\file52", path);
        assert!(had_root);

        let (path, had_root) = strip_absolute_path_bsdtar("D:../file05");
        assert_eq!("../file05", path);
        assert!(had_root);

        let (path, had_root) = strip_absolute_path_bsdtar("c:../..\\file43");
        assert_eq!("../..\\file43", path);
        assert!(had_root);
    }

    #[test]
    fn has_parent_dir_component_detects_windows_style_paths() {
        assert!(has_parent_dir_component("..\\file37"));
        assert!(has_parent_dir_component("../..\\file43"));
        let (rewritten, _) = strip_absolute_path_bsdtar("\\\\?\\UNC\\..\\file37");
        assert!(has_parent_dir_component(rewritten));
        assert!(!has_parent_dir_component("server\\share\\file35"));
    }

    #[test]
    fn editor_preserve_curdir_handles_windows_style_absolute_paths() {
        let editor = PathnameEditor::new(None, None, false, true);
        #[cfg(windows)]
        let expected_unc_backslash = "server/share/file35";
        #[cfg(not(windows))]
        let expected_unc_backslash = "server\\share\\file35";

        #[cfg(windows)]
        let expected_mixed_unc_backslash = "server/share/file52";
        #[cfg(not(windows))]
        let expected_mixed_unc_backslash = "server\\share\\file52";

        assert_eq!(
            "file04",
            editor
                .edit_entry_name(Path::new("c:/file04"))
                .unwrap()
                .as_str()
        );
        assert_eq!(
            "server/share/file15",
            editor
                .edit_entry_name(Path::new("//?/UNC/server/share/file15"))
                .unwrap()
                .as_str()
        );
        assert_eq!(
            expected_unc_backslash,
            editor
                .edit_entry_name(Path::new("\\\\?\\UNC\\server\\share\\file35"))
                .unwrap()
                .as_str()
        );
        assert_eq!(
            expected_mixed_unc_backslash,
            editor
                .edit_entry_name(Path::new("\\/?/uNc/server\\share\\file52"))
                .unwrap()
                .as_str()
        );
    }

    #[test]
    fn editor_preserve_curdir_rejects_windows_style_parent_dir_paths() {
        let editor = PathnameEditor::new(None, None, false, true);
        assert!(editor.edit_entry_name(Path::new("D:../file05")).is_none());
        assert!(
            editor
                .edit_entry_name(Path::new("\\\\?\\UNC\\..\\file37"))
                .is_none()
        );
        assert!(
            editor
                .edit_entry_name(Path::new("c:../..\\file43"))
                .is_none()
        );
        assert!(
            editor
                .edit_entry_name(Path::new("\\/?\\UnC\\../file54"))
                .is_none()
        );
    }

    #[test]
    fn editor_preserve_curdir_hardlink_tracks_windows_root_stripping() {
        let editor = PathnameEditor::new(None, None, false, true);
        let (reference, had_root) = editor.edit_hardlink(Path::new("c:/etc/hosts")).unwrap();
        assert_eq!(reference.as_str(), "etc/hosts");
        assert!(had_root);
    }

    // --- B1: strip_absolute_path_bsdtar boundary values ---

    #[test]
    fn strip_absolute_path_bsdtar_empty_string() {
        let (path, had_root) = strip_absolute_path_bsdtar("");
        assert_eq!("", path);
        assert!(!had_root);
    }

    #[test]
    fn strip_absolute_path_bsdtar_single_forward_slash() {
        let (path, had_root) = strip_absolute_path_bsdtar("/");
        assert_eq!("", path);
        assert!(had_root);
    }

    #[test]
    fn strip_absolute_path_bsdtar_single_backslash() {
        let (path, had_root) = strip_absolute_path_bsdtar("\\");
        assert_eq!("", path);
        assert!(had_root);
    }

    #[test]
    fn strip_absolute_path_bsdtar_multiple_separators() {
        let (path, had_root) = strip_absolute_path_bsdtar("///");
        assert_eq!("", path);
        assert!(had_root);

        let (path, had_root) = strip_absolute_path_bsdtar("\\\\");
        assert_eq!("", path);
        assert!(had_root);
    }

    #[test]
    fn strip_absolute_path_bsdtar_drive_letter_only() {
        let (path, had_root) = strip_absolute_path_bsdtar("c:");
        assert_eq!("", path);
        assert!(had_root);
    }

    #[test]
    fn strip_absolute_path_bsdtar_terminal_dotdot() {
        // bsdtar: "/.."-at-end -- p[3]=='\0' is not a separator, so only "/"
        // is stripped, leaving "..". The ".." is caught by has_parent_dir_component.
        let (path, had_root) = strip_absolute_path_bsdtar("/..");
        assert_eq!("..", path);
        assert!(had_root);

        let (path, had_root) = strip_absolute_path_bsdtar("\\..");
        assert_eq!("..", path);
        assert!(had_root);
    }

    #[test]
    fn strip_absolute_path_bsdtar_terminal_dot() {
        // bsdtar: "/."-at-end -- p[2]=='\0' is not a separator, so only "/"
        // is stripped, leaving ".".
        let (path, had_root) = strip_absolute_path_bsdtar("/.");
        assert_eq!(".", path);
        assert!(had_root);

        let (path, had_root) = strip_absolute_path_bsdtar("\\.");
        assert_eq!(".", path);
        assert!(had_root);
    }

    #[test]
    fn strip_absolute_path_bsdtar_prefix_exactly_4_bytes() {
        let (path, had_root) = strip_absolute_path_bsdtar("//?/");
        assert_eq!("", path);
        assert!(had_root);
    }

    #[test]
    fn strip_absolute_path_bsdtar_unc_prefix_exactly_8_bytes() {
        let (path, had_root) = strip_absolute_path_bsdtar("//?/UNC/");
        assert_eq!("", path);
        assert!(had_root);
    }

    #[test]
    fn strip_absolute_path_bsdtar_safe_relative_paths() {
        let (path, had_root) = strip_absolute_path_bsdtar("file.txt");
        assert_eq!("file.txt", path);
        assert!(!had_root);

        let (path, had_root) = strip_absolute_path_bsdtar("a/b/c");
        assert_eq!("a/b/c", path);
        assert!(!had_root);

        let (path, had_root) = strip_absolute_path_bsdtar("./a/b");
        assert_eq!("./a/b", path);
        assert!(!had_root);
    }

    #[test]
    fn strip_absolute_path_bsdtar_leading_separator_with_content() {
        let (path, had_root) = strip_absolute_path_bsdtar("/file");
        assert_eq!("file", path);
        assert!(had_root);

        let (path, had_root) = strip_absolute_path_bsdtar("\\file");
        assert_eq!("file", path);
        assert!(had_root);
    }

    // --- B2: device prefix (\\.\) tests ---

    #[test]
    fn strip_absolute_path_bsdtar_device_prefix_with_drive() {
        // \\.\C:\file -- device prefix (4 bytes stripped), then drive letter
        let (path, had_root) = strip_absolute_path_bsdtar("\\\\.\\C:\\file");
        assert_eq!("file", path);
        assert!(had_root);

        // //./C:/file -- forward-slash variant
        let (path, had_root) = strip_absolute_path_bsdtar("//./C:/file");
        assert_eq!("file", path);
        assert!(had_root);
    }

    #[test]
    fn strip_absolute_path_bsdtar_device_prefix_only() {
        // \\.\ alone (4 bytes + trailing separator)
        let (path, had_root) = strip_absolute_path_bsdtar("\\\\.\\");
        assert_eq!("", path);
        assert!(had_root);
    }

    #[test]
    fn strip_absolute_path_bsdtar_device_unc_prefix() {
        // \\.\UNC\ is treated as a 4-byte API prefix (\\.\), NOT as an 8-byte
        // UNC prefix. Only \\?\ triggers the UNC check (matches_unc_api_prefix
        // requires bytes[2] == b'?'). After stripping \\.\, "UNC\..." remains
        // and "UNC" is not a drive letter, so it stays.
        let (path, had_root) = strip_absolute_path_bsdtar("\\\\.\\UNC\\server\\share\\file");
        assert_eq!("UNC\\server\\share\\file", path);
        assert!(had_root);
    }

    #[test]
    fn strip_absolute_path_bsdtar_device_prefix_with_traversal() {
        // \\.\..\secret -- device prefix (4 bytes) stripped, "..\secret" remains.
        // The ".." is NOT consumed because it doesn't follow a separator;
        // has_parent_dir_component catches it downstream.
        let (path, had_root) = strip_absolute_path_bsdtar("\\\\.\\..\\secret");
        assert_eq!("..\\secret", path);
        assert!(had_root);
        assert!(has_parent_dir_component(path));
    }

    // --- B3+B4: is_unsafe_link equivalent (strip + has_parent_dir combined) ---
    // is_unsafe_link is: had_root || has_parent_dir_component(rewritten)
    // We test the same logic directly here.

    #[test]
    fn strip_then_parent_dir_detects_drive_dotdot_backslash() {
        let (rewritten, had_root) = strip_absolute_path_bsdtar("c:..\\file");
        assert!(had_root, "drive letter should be stripped");
        assert!(has_parent_dir_component(rewritten), ".. should be detected");
    }

    #[test]
    fn strip_then_parent_dir_detects_slash_dotdot_backslash() {
        // /..\ is consumed as /../ equivalent (both separators recognized),
        // so .. does not remain — had_root alone catches it.
        let (rewritten, had_root) = strip_absolute_path_bsdtar("/..\\file");
        assert!(had_root, "leading slash and /..\\ should be stripped");
        assert_eq!("file", rewritten);
    }

    #[test]
    fn strip_then_parent_dir_detects_drive_slash_dotdot_backslash() {
        // c:/../ is fully consumed: drive letter, slash, and /../ sequence.
        let (rewritten, had_root) = strip_absolute_path_bsdtar("c:/..\\file");
        assert!(had_root, "drive letter, slash and /..\\ should be stripped");
        assert_eq!("file", rewritten);
    }

    #[test]
    fn strip_then_parent_dir_detects_device_prefix_dotdot() {
        let (rewritten, had_root) = strip_absolute_path_bsdtar("\\\\?\\..\\file");
        assert!(had_root, "device prefix should be stripped");
        assert!(has_parent_dir_component(rewritten), ".. should be detected");
    }

    #[test]
    fn strip_then_parent_dir_allows_safe_path() {
        let (rewritten, had_root) = strip_absolute_path_bsdtar("a/b/c");
        assert!(!had_root);
        assert!(!has_parent_dir_component(rewritten));
    }

    #[test]
    fn is_unsafe_link_path_detects_all_unsafe_patterns() {
        // Windows drive prefix
        assert!(is_unsafe_link_path("C:/file"));
        // POSIX absolute
        assert!(is_unsafe_link_path("/etc/passwd"));
        // Backslash parent traversal
        assert!(is_unsafe_link_path("..\\file"));
        // Forward slash parent traversal
        assert!(is_unsafe_link_path("../file"));
        // Windows UNC with embedded dotdot
        assert!(is_unsafe_link_path("\\\\?\\UNC\\..\\file"));
        // Drive-relative with dotdot
        assert!(is_unsafe_link_path("D:../file"));
    }

    #[test]
    fn is_unsafe_link_path_allows_safe_patterns() {
        assert!(!is_unsafe_link_path("a/b/c"));
        assert!(!is_unsafe_link_path("file.txt"));
        assert!(!is_unsafe_link_path("./a/b"));
    }

    #[cfg(unix)]
    #[test]
    fn rewrite_path_strips_prefix_from_non_utf8_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        // b"/\xff/file" — leading '/' + non-UTF-8 byte + "/file"
        let path = Path::new(OsStr::from_bytes(b"/\xff/file"));
        let editor = PathnameEditor::new(None, None, false, true);
        let rewritten = editor.rewrite_path_for_extraction(path);
        assert!(rewritten.had_root);
        assert!(rewritten.path.contains("file"));
        assert!(!rewritten.path.starts_with('/'));
    }

    #[test]
    fn has_parent_dir_component_adversarial_cases() {
        // mid-path backslash dotdot
        assert!(has_parent_dir_component("foo\\..\\bar"));
        // standalone dotdot
        assert!(has_parent_dir_component(".."));
        // dotdot with mixed separators
        assert!(has_parent_dir_component("a/b\\../c"));
        // trailing separator after dotdot
        assert!(has_parent_dir_component("../"));
        // NOT parent dir: "..name" is a normal component
        assert!(!has_parent_dir_component("a/..name"));
        // NOT parent dir: "..." is a normal component
        assert!(!has_parent_dir_component("a/.../b"));
    }

    #[test]
    fn editor_junction_preserves_unix_absolute() {
        let editor = PathnameEditor::new(None, None, false, false);
        let out = editor.edit_junction(Path::new("/abs/target"));
        assert_eq!(out.as_str(), "/abs/target");
    }

    #[test]
    fn editor_junction_preserves_windows_absolute() {
        let editor = PathnameEditor::new(None, None, false, false);
        let out = editor.edit_junction(Path::new("C:\\abs\\target"));
        assert_eq!(out.as_str(), "C:\\abs\\target");
    }

    #[test]
    fn editor_junction_preserves_relative_unchanged() {
        let editor = PathnameEditor::new(None, None, false, false);
        let out = editor.edit_junction(Path::new("rel/target"));
        assert_eq!(out.as_str(), "rel/target");
    }

    #[test]
    fn editor_junction_does_not_apply_strip_components() {
        let editor = PathnameEditor::new(Some(1), None, false, false);
        let out = editor.edit_junction(Path::new("/abs/target"));
        // strip_components does NOT apply to junction targets, matching symlink semantics.
        assert_eq!(out.as_str(), "/abs/target");
    }
}
