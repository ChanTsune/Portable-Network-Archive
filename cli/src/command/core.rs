pub(crate) mod path;
mod path_filter;
pub(crate) mod path_lock;
mod path_transformer;
pub(crate) mod permission;
pub(crate) mod re;
pub(crate) mod safe_writer;
pub(crate) mod time_filter;
pub(crate) mod timestamp;

pub(crate) use self::path::PathnameEditor;
pub(crate) use self::permission::{ModeStrategy, OwnerOptions, OwnerStrategy};
pub(crate) use self::safe_writer::SafeWriter;
pub(crate) use self::timestamp::{TimeSource, TimestampStrategy};
use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs, HashAlgorithmArgs},
    utils::{self, PathPartExt, fs::HardlinkResolver},
};
use anyhow::Context;
pub(crate) use path_filter::PathFilter;
use path_slash::*;
pub(crate) use path_transformer::PathTransformers;
use pna::{
    Archive, EntryBuilder, EntryPart, MIN_CHUNK_BYTES_SIZE, NormalEntry, PNA_HEADER, ReadEntry,
    SolidEntryBuilder, WriteOptions, prelude::*,
};
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt, fs,
    io::{self, prelude::*},
    path::{Path, PathBuf},
    time::SystemTime,
};
pub(crate) use time_filter::{TimeFilter, TimeFilters};

/// Options controlling how filesystem items are collected for archiving.
///
/// This struct groups all traversal and filtering options that were previously
/// passed as individual parameters.
#[derive(Clone, Debug)]
pub(crate) struct CollectOptions<'a> {
    pub(crate) recursive: bool,
    pub(crate) keep_dir: bool,
    pub(crate) gitignore: bool,
    pub(crate) nodump: bool,
    pub(crate) follow_links: bool,
    pub(crate) follow_command_links: bool,
    pub(crate) one_file_system: bool,
    pub(crate) filter: &'a PathFilter<'a>,
    pub(crate) time_filters: &'a TimeFilters,
}

/// Resolves CLI time filter options into a `TimeFilters` instance.
/// Path arguments (`*_than`) take precedence over direct `SystemTime` values.
pub(crate) struct TimeFilterResolver<'a> {
    pub(crate) newer_ctime_than: Option<&'a Path>,
    pub(crate) older_ctime_than: Option<&'a Path>,
    pub(crate) newer_ctime: Option<SystemTime>,
    pub(crate) older_ctime: Option<SystemTime>,
    pub(crate) newer_mtime_than: Option<&'a Path>,
    pub(crate) older_mtime_than: Option<&'a Path>,
    pub(crate) newer_mtime: Option<SystemTime>,
    pub(crate) older_mtime: Option<SystemTime>,
}

impl TimeFilterResolver<'_> {
    /// Resolves file paths and times into a `TimeFilters` instance.
    pub(crate) fn resolve(self) -> io::Result<TimeFilters> {
        fn resolve_ctime(path: &Path) -> io::Result<SystemTime> {
            fs::metadata(path)?.created().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!(
                        "creation time (birth time) is not available for '{}'",
                        path.display()
                    ),
                )
            })
        }

        Ok(TimeFilters {
            ctime: TimeFilter {
                newer_than: match self.newer_ctime_than {
                    Some(p) => Some(resolve_ctime(p)?),
                    None => self.newer_ctime,
                },
                older_than: match self.older_ctime_than {
                    Some(p) => Some(resolve_ctime(p)?),
                    None => self.older_ctime,
                },
            },
            mtime: TimeFilter {
                newer_than: match self.newer_mtime_than {
                    Some(p) => Some(fs::metadata(p)?.modified()?),
                    None => self.newer_mtime,
                },
                older_than: match self.older_mtime_than {
                    Some(p) => Some(fs::metadata(p)?.modified()?),
                    None => self.older_mtime,
                },
            },
        })
    }
}

/// Overhead for a split archive part in bytes, including PNA header, AHED, ANXT, and AEND chunks.
pub(crate) const SPLIT_ARCHIVE_OVERHEAD_BYTES: usize =
    PNA_HEADER.len() + MIN_CHUNK_BYTES_SIZE * 3 + 8;

/// Minimum bytes required for a split archive part (overhead + one minimal chunk).
pub(crate) const MIN_SPLIT_PART_BYTES: usize = SPLIT_ARCHIVE_OVERHEAD_BYTES + MIN_CHUNK_BYTES_SIZE;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum XattrStrategy {
    Never,
    Always,
}

impl XattrStrategy {
    pub(crate) const fn from_flags(keep_xattr: bool, no_keep_xattr: bool) -> Self {
        if no_keep_xattr {
            Self::Never
        } else if keep_xattr {
            Self::Always
        } else {
            Self::Never
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum AclStrategy {
    Never,
    Always,
}

impl AclStrategy {
    pub(crate) const fn from_flags(keep_acl: bool, no_keep_acl: bool) -> Self {
        if no_keep_acl {
            Self::Never
        } else if keep_acl {
            Self::Always
        } else {
            Self::Never
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum FflagsStrategy {
    Never,
    Always,
}

impl FflagsStrategy {
    pub(crate) const fn from_flags(keep_fflags: bool, no_keep_fflags: bool) -> Self {
        if no_keep_fflags {
            Self::Never
        } else if keep_fflags {
            Self::Always
        } else {
            Self::Never
        }
    }
}

/// Strategy for handling macOS metadata in AppleDouble format.
/// When enabled, creates `._` prefixed entries containing AppleDouble data
/// (extended attributes, ACLs, resource forks) for each file with Mac metadata.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum MacMetadataStrategy {
    /// Do not create AppleDouble entries (default)
    #[default]
    Never,
    /// Create AppleDouble entries for files with Mac metadata (macOS only)
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    Always,
}

impl MacMetadataStrategy {
    /// Creates a strategy from CLI flags, considering platform.
    /// On non-macOS platforms, always returns Never regardless of flags.
    #[cfg(target_os = "macos")]
    pub(crate) const fn from_flags(mac_metadata: bool, no_mac_metadata: bool) -> Self {
        if no_mac_metadata {
            Self::Never
        } else if mac_metadata {
            Self::Always
        } else {
            Self::Never
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) const fn from_flags(_mac_metadata: bool, _no_mac_metadata: bool) -> Self {
        Self::Never
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct KeepOptions {
    pub(crate) timestamp_strategy: TimestampStrategy,
    pub(crate) mode_strategy: ModeStrategy,
    pub(crate) owner_strategy: OwnerStrategy,
    pub(crate) xattr_strategy: XattrStrategy,
    pub(crate) acl_strategy: AclStrategy,
    pub(crate) fflags_strategy: FflagsStrategy,
    pub(crate) mac_metadata_strategy: MacMetadataStrategy,
}

/// Resolves CLI timestamp options into a `TimestampStrategy`.
///
/// This struct encapsulates the CLI-specific logic for determining timestamp behavior:
/// - `no_keep_timestamp` forces `NoPreserve`
/// - `keep_timestamp` or any time override enables `Preserve`
/// - `default_preserve` determines behavior when no flags are specified
pub(crate) struct TimestampStrategyResolver {
    pub(crate) keep_timestamp: bool,
    pub(crate) no_keep_timestamp: bool,
    pub(crate) default_preserve: bool,
    pub(crate) mtime: Option<SystemTime>,
    pub(crate) clamp_mtime: bool,
    pub(crate) ctime: Option<SystemTime>,
    pub(crate) clamp_ctime: bool,
    pub(crate) atime: Option<SystemTime>,
    pub(crate) clamp_atime: bool,
}

impl TimestampStrategyResolver {
    fn time_source_from(value: Option<SystemTime>, clamp: bool) -> TimeSource {
        match (value, clamp) {
            (Some(t), true) => TimeSource::ClampTo(t),
            (Some(t), false) => TimeSource::Override(t),
            (None, _) => TimeSource::FromSource,
        }
    }

    fn has_any_override(&self) -> bool {
        self.mtime.is_some() || self.ctime.is_some() || self.atime.is_some()
    }

    /// Resolves CLI options into a `TimestampStrategy`.
    pub(crate) fn resolve(self) -> TimestampStrategy {
        if self.no_keep_timestamp {
            TimestampStrategy::NoPreserve
        } else if self.keep_timestamp || self.has_any_override() {
            TimestampStrategy::Preserve {
                mtime: Self::time_source_from(self.mtime, self.clamp_mtime),
                ctime: Self::time_source_from(self.ctime, self.clamp_ctime),
                atime: Self::time_source_from(self.atime, self.clamp_atime),
            }
        } else if self.default_preserve {
            TimestampStrategy::preserve()
        } else {
            TimestampStrategy::NoPreserve
        }
    }
}

/// Resolves CLI permission options into split mode and owner strategies.
///
/// This struct encapsulates the CLI-specific logic for determining permission behavior:
/// - `no_keep_permission` forces both mode and owner to `Never`
/// - `keep_permission` enables `Preserve` for mode, and ownership handling via `same_owner`
/// - `same_owner` controls whether to restore ownership (extraction only)
/// - Owner override fields (uid, gid, uname, gname) are used in both creation and extraction
pub(crate) struct PermissionStrategyResolver {
    pub(crate) keep_permission: bool,
    pub(crate) no_keep_permission: bool,
    pub(crate) same_owner: bool,
    pub(crate) uname: Option<String>,
    pub(crate) gname: Option<String>,
    pub(crate) uid: Option<u32>,
    pub(crate) gid: Option<u32>,
    pub(crate) numeric_owner: bool,
}

impl PermissionStrategyResolver {
    /// Resolves CLI options to split (ModeStrategy, OwnerStrategy).
    ///
    /// The `same_owner` field controls ownership handling:
    /// - `true`: Restore/store ownership (Preserve with options)
    /// - `false`: Skip ownership restoration (Never)
    ///
    /// For creation contexts, pass `same_owner: true` since ownership
    /// is always stored when `--keep-permission` is enabled.
    pub(crate) fn resolve(self) -> (ModeStrategy, OwnerStrategy) {
        if self.no_keep_permission {
            (ModeStrategy::Never, OwnerStrategy::Never)
        } else if self.keep_permission {
            let mode_strategy = ModeStrategy::Preserve;
            let owner_strategy = if self.same_owner {
                OwnerStrategy::Preserve {
                    options: OwnerOptions {
                        uname: if self.numeric_owner {
                            Some(String::new())
                        } else {
                            self.uname
                        },
                        gname: if self.numeric_owner {
                            Some(String::new())
                        } else {
                            self.gname
                        },
                        uid: self.uid,
                        gid: self.gid,
                    },
                }
            } else {
                OwnerStrategy::Never
            };
            (mode_strategy, owner_strategy)
        } else {
            (ModeStrategy::Never, OwnerStrategy::Never)
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CreateOptions {
    pub(crate) option: WriteOptions,
    pub(crate) keep_options: KeepOptions,
    pub(crate) pathname_editor: PathnameEditor,
}

/// Gitignore-style exclusion rules.
struct Ignore {
    // Map of directory path -> compiled .gitignore matcher for that directory
    by_dir: HashMap<PathBuf, ignore::gitignore::Gitignore>,
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
            let (ig, _) = ignore::gitignore::Gitignore::new(&gitignore_path);
            // Key by the directory that owns this .gitignore
            self.by_dir.insert(path.to_path_buf(), ig);
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CollectedEntry {
    pub(crate) path: PathBuf,
    pub(crate) store_as: StoreAs,
    pub(crate) metadata: fs::Metadata,
}

#[derive(Clone, Debug)]
pub(crate) enum StoreAs {
    File,
    Dir,
    Symlink,
    Hardlink(PathBuf),
}

/// Source of an archive to include (file path or stdin).
#[derive(Clone, Debug)]
pub(crate) enum ArchiveSource {
    File(PathBuf),
    Stdin,
}

impl fmt::Display for ArchiveSource {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File(path) => fmt::Display::fmt(&path.display(), f),
            Self::Stdin => f.write_str("-"),
        }
    }
}

/// Represents a CLI file argument that can be either a filesystem path or an archive inclusion.
///
/// Archive inclusions start with '@' and reference entries from an existing archive.
/// This follows bsdtar's convention for including archives.
#[derive(Clone, Debug)]
pub(crate) enum ItemSource {
    /// A regular filesystem path (file or directory).
    Filesystem(PathBuf),
    /// An archive to include entries from.
    Archive(ArchiveSource),
}

impl ItemSource {
    /// Parses a single CLI argument into an `ItemSource`.
    ///
    /// - `@` or `@-` → `Archive(Stdin)`
    /// - `@path` → `Archive(File(path))`
    /// - `path` → `Filesystem(path)`
    ///
    /// To include a file whose name starts with `@`, use `./@filename` (bsdtar convention).
    ///
    /// Paths are stored as-is. Both filesystem and archive paths are resolved
    /// relative to the current working directory at the time they are accessed,
    /// which means they are affected by the -C option.
    pub(crate) fn parse(arg: &str) -> Self {
        if let Some(archive_path) = arg.strip_prefix('@') {
            if archive_path.is_empty() || archive_path == "-" {
                Self::Archive(ArchiveSource::Stdin)
            } else {
                Self::Archive(ArchiveSource::File(PathBuf::from(archive_path)))
            }
        } else {
            Self::Filesystem(PathBuf::from(arg))
        }
    }

    /// Parses multiple CLI arguments into `ItemSource` values.
    pub(crate) fn parse_many(args: &[String]) -> Vec<Self> {
        args.iter().map(|s| Self::parse(s)).collect()
    }
}

/// Validates that stdin is not used as a source more than once.
///
/// Returns an error if multiple `@-` or `@` sources are found,
/// since stdin can only be read once.
pub(crate) fn validate_no_duplicate_stdin(sources: &[ItemSource]) -> anyhow::Result<()> {
    let stdin_count = sources
        .iter()
        .filter(|s| matches!(s, ItemSource::Archive(ArchiveSource::Stdin)))
        .count();
    if stdin_count > 1 {
        anyhow::bail!("stdin (@- or @) can only be specified once as an archive source");
    }
    Ok(())
}

/// Validates that stdin is not used as a source when output is going to stdout.
///
/// When `-f -` is used (writing archive to stdout), stdin cannot also be used
/// as an archive source (`@-` or `@`), since both would need exclusive access
/// to the standard streams.
pub(crate) fn validate_no_stdin_stdout_conflict(
    sources: &[ItemSource],
    output_is_stdout: bool,
) -> anyhow::Result<()> {
    if output_is_stdout {
        let uses_stdin = sources
            .iter()
            .any(|s| matches!(s, ItemSource::Archive(ArchiveSource::Stdin)));
        if uses_stdin {
            anyhow::bail!(
                "cannot use stdin as archive source (@- or @) when outputting to stdout (-f -)"
            );
        }
    }
    Ok(())
}

/// Represents a collected item ready for archive creation.
///
/// This preserves the CLI argument order while separating filesystem items
/// (which need entry building) from archive markers (which need entry copying).
#[derive(Clone, Debug)]
pub(crate) enum CollectedItem {
    /// A filesystem item with its path and storage strategy.
    Filesystem(CollectedEntry),
    /// A marker indicating where to insert entries from an archive source.
    ArchiveMarker(ArchiveSource),
}

/// Result type for entries sent through the channel.
///
/// This enum allows batching multiple archive entries into a single channel send
/// operation, reducing synchronization overhead compared to sending each entry
/// individually.
#[allow(clippy::large_enum_variant)]
pub(crate) enum EntryResult {
    /// A single entry from a filesystem item.
    Single(io::Result<Option<NormalEntry>>),
    /// A batch of entries from an archive source.
    Batch(io::Result<Vec<io::Result<Option<NormalEntry>>>>),
}

impl EntryResult {
    pub(crate) fn into_entries(self) -> Vec<io::Result<Option<NormalEntry>>> {
        match self {
            EntryResult::Single(entry) => vec![entry],
            EntryResult::Batch(entries) => entries.unwrap_or_else(|e| vec![Err(e)]),
        }
    }
}

/// Drains entry results and applies a callback to each emitted entry.
pub(crate) fn drain_entry_results<I, F, T>(results: I, mut add_entry: F) -> io::Result<()>
where
    I: IntoIterator<Item = EntryResult>,
    F: FnMut(NormalEntry) -> io::Result<T>,
{
    for result in results {
        match result {
            EntryResult::Single(entry) => {
                if let Some(entry) = entry? {
                    add_entry(entry)?;
                }
            }
            EntryResult::Batch(entries) => {
                for entry in entries? {
                    if let Some(entry) = entry? {
                        add_entry(entry)?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Spawns entry creation for filesystem items and reads archive sources.
pub(crate) fn spawn_entry_results(
    target_items: Vec<CollectedItem>,
    create_options: &CreateOptions,
    filter: &PathFilter<'_>,
    time_filters: &TimeFilters,
    password: Option<&[u8]>,
) -> std::sync::mpsc::Receiver<EntryResult> {
    let (tx, rx) = std::sync::mpsc::channel();
    rayon::scope_fifo(|s| {
        for item in target_items {
            match item {
                CollectedItem::Filesystem(entry) => {
                    let tx = tx.clone();
                    s.spawn_fifo(move |_| {
                        log::debug!("Adding: {}", entry.path.display());
                        tx.send(EntryResult::Single(create_entry(&entry, create_options)))
                            .unwrap_or_else(|e| log::error!("{e}: {}", entry.path.display()));
                    })
                }
                CollectedItem::ArchiveMarker(source) => {
                    let result = read_archive_source(
                        &source,
                        create_options,
                        filter,
                        time_filters,
                        password,
                    );
                    tx.send(EntryResult::Batch(result))
                        .unwrap_or_else(|e| log::error!("{e}: archive source {}", source));
                }
            }
        }

        drop(tx);
    });
    rx
}

/// Collects items from mixed filesystem and archive sources, preserving order.
///
/// For filesystem sources, uses the existing collection logic with shared
/// hardlink detection. For archive sources, returns markers that indicate
/// where archive entries should be inserted.
///
/// # Order Guarantee
/// - Between arguments: strictly preserved
/// - Within a single filesystem argument: walkdir traversal order
///
/// # Hardlink Detection
/// A single `HardlinkResolver` is shared across all filesystem paths,
/// enabling cross-path hardlink detection.
pub(crate) fn collect_items_from_sources(
    sources: impl IntoIterator<Item = ItemSource>,
    options: &CollectOptions<'_>,
    hardlink_resolver: &mut HardlinkResolver,
) -> io::Result<Vec<CollectedItem>> {
    let mut results = Vec::new();

    for source in sources {
        match source {
            ItemSource::Filesystem(path) => {
                let items = collect_items_with_state(&path, options, hardlink_resolver)?;
                results.extend(items.into_iter().map(CollectedItem::Filesystem));
            }
            ItemSource::Archive(archive_source) => {
                results.push(CollectedItem::ArchiveMarker(archive_source));
            }
        }
    }

    Ok(results)
}

/// Collects items from multiple paths, preserving CLI argument order.
///
/// State such as hardlink detection is shared across all paths via the provided
/// `HardlinkResolver`, enabling cross-path hardlink recognition. The resolver
/// can be inspected after collection to check for incomplete hardlink sets via
/// [`HardlinkResolver::incomplete_links`].
///
/// # Order Preservation
/// Items are collected in the order paths are provided. Each path's items
/// appear in traversal order. This enables predictable archive ordering
/// matching CLI argument order.
pub(crate) fn collect_items_from_paths<P: AsRef<Path>>(
    paths: impl IntoIterator<Item = P>,
    options: &CollectOptions<'_>,
    hardlink_resolver: &mut HardlinkResolver,
) -> io::Result<Vec<CollectedEntry>> {
    let mut results = Vec::new();
    for path in paths {
        results.extend(collect_items_with_state(
            path.as_ref(),
            options,
            hardlink_resolver,
        )?);
    }
    Ok(results)
}

/// Walks a single path and collects filesystem items to archive.
///
/// Returns a list of [`CollectedEntry`] indicating how each discovered item
/// should be stored in the archive (`StoreAs`), along with its pre-captured
/// metadata. Traversal supports recursion, exclusion filters, and correct
/// handling of symbolic and hard links.
///
/// Behavior summary:
/// - Recursion: when `options.recursive` is true, subdirectories are walked
///   recursively; otherwise only the provided path is inspected.
/// - Directory entries: when `options.keep_dir` is true, directories are included as
///   `StoreAs::Dir`. When false, directories act only as containers during
///   traversal and are not returned themselves.
/// - Exclusion: entries whose slash-separated path matches `options.filter` exclusions
///   are pruned from the traversal and not returned.
/// - Gitignore pruning: when `options.gitignore` is true, `.gitignore` files found in
///   encountered directories are loaded and applied using closest-precedence
///   rules. Ignored files are skipped; ignored directories are pruned from
///   descent. Patterns are evaluated relative to the directory that owns the
///   `.gitignore`.
/// - Symbolic links: by default, symlinks are returned as `StoreAs::Symlink`.
///   If `options.follow_links` is true, symlinks are resolved and classified by their
///   targets (file => `StoreAs::File`, dir => `StoreAs::Dir`). If
///   `options.follow_command_links` is true, only the top-level input path (depth 0)
///   is followed; nested symlinks require `follow_links`. Broken symlinks are
///   still returned as `StoreAs::Symlink`.
/// - Hard links: regular files detected as hard links to a previously seen
///   file are returned as `StoreAs::Hardlink(<target>)`, where `<target>` is the
///   canonical path of the first occurrence; otherwise they are returned as
///   `StoreAs::File`.
/// - One File System: when `options.one_file_system` is true, the traversal will not
///   cross filesystem boundaries.
/// - Unsupported types: special files that are neither regular files, dirs, nor
///   symlinks produce an `io::ErrorKind::Unsupported` error.
///
/// This function accepts a shared `HardlinkResolver` to enable cross-path hardlink
/// detection when collecting from multiple paths.
///
/// Returns a vector of [`CollectedEntry`] on success.
///
/// # Errors
/// Propagates I/O errors encountered during traversal. Broken symlinks are
/// tolerated and returned as `StoreAs::Symlink` instead of an error. Returns
/// `io::ErrorKind::Unsupported` for entries with unsupported types. Other walk
/// errors are wrapped using `io::Error::other`.
pub(crate) fn collect_items_with_state(
    path: &Path,
    options: &CollectOptions<'_>,
    hardlink_resolver: &mut HardlinkResolver,
) -> io::Result<Vec<CollectedEntry>> {
    let mut ig = Ignore::empty();
    let mut out = Vec::new();

    let mut iter = if options.recursive {
        walkdir::WalkDir::new(path)
    } else {
        walkdir::WalkDir::new(path).max_depth(0)
    }
    .follow_links(options.follow_links)
    .follow_root_links(options.follow_command_links)
    .same_file_system(options.one_file_system)
    .into_iter();

    while let Some(res) = iter.next() {
        match res {
            Ok(entry) => {
                let path = entry.path();
                let ty = entry.file_type();
                let depth = entry.depth();
                let should_follow =
                    options.follow_links || (depth == 0 && options.follow_command_links);
                let is_dir = ty.is_dir() || (ty.is_symlink() && should_follow && path.is_dir());
                let is_file = ty.is_file() || (ty.is_symlink() && should_follow && path.is_file());
                let is_symlink = ty.is_symlink() && !should_follow;

                // Exclude (prunes descent when directory)
                if options.filter.excluded(path.to_slash_lossy()) {
                    if is_dir {
                        iter.skip_current_dir();
                    }
                    continue;
                }

                if options.gitignore {
                    // Gitignore pruning before reading this dir's .gitignore
                    if ig.is_ignore(path, is_dir) {
                        if is_dir {
                            iter.skip_current_dir();
                        }
                        continue;
                    }
                    // After confirming not ignored, load .gitignore from this directory
                    if is_dir {
                        ig.add_path(path);
                    }
                }

                if options.nodump {
                    match utils::fs::is_nodump(path) {
                        Ok(true) => {
                            if is_dir {
                                iter.skip_current_dir();
                            }
                            continue;
                        }
                        Ok(false) => {}
                        Err(e) => {
                            log::warn!("Failed to check nodump flag for {}: {}", path.display(), e);
                        }
                    }
                }

                // Classify entry and maybe add it to output
                let store = if is_symlink {
                    Some((StoreAs::Symlink, fs::symlink_metadata(path)?))
                } else if is_file {
                    if let Some(linked) = hardlink_resolver.resolve(path).ok().flatten() {
                        Some((StoreAs::Hardlink(linked), fs::symlink_metadata(path)?))
                    } else {
                        Some((StoreAs::File, fs::metadata(path)?))
                    }
                } else if is_dir {
                    if options.keep_dir {
                        Some((StoreAs::Dir, fs::metadata(path)?))
                    } else {
                        None
                    }
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!("Unsupported file type: {}", path.display()),
                    ));
                };

                if let Some((store_as, metadata)) = store {
                    if options
                        .time_filters
                        .matches_or_inactive(metadata.created().ok(), metadata.modified().ok())
                    {
                        out.push(CollectedEntry {
                            path: path.to_path_buf(),
                            store_as,
                            metadata,
                        });
                    }
                }
            }
            Err(e) => {
                if let Some(ioe) = e.io_error() {
                    if let Some(path) = e.path() {
                        let metadata = fs::symlink_metadata(path)?;
                        if is_broken_symlink_error(&metadata, ioe) {
                            out.push(CollectedEntry {
                                path: path.to_path_buf(),
                                store_as: StoreAs::Symlink,
                                metadata,
                            });
                            continue;
                        }
                    }
                }
                return Err(io::Error::other(e));
            }
        }
    }
    Ok(out)
}

#[inline]
fn is_broken_symlink_error(meta: &fs::Metadata, err: &io::Error) -> bool {
    meta.is_symlink() && err.kind() == io::ErrorKind::NotFound
}

pub(crate) fn collect_split_archives(first: impl AsRef<Path>) -> io::Result<Vec<fs::File>> {
    let first = first.as_ref();
    let mut archives = Vec::new();
    let mut n = 1;
    let mut target_archive = Cow::from(first);
    while fs::exists(&target_archive)? {
        archives.push(fs::File::open(&target_archive)?);
        n += 1;
        target_archive = target_archive.with_part(n).into();
    }
    if archives.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("No archive found at {}", first.display()),
        ));
    }
    Ok(archives)
}

const IN_MEMORY_THRESHOLD: usize = 50 * 1024 * 1024;

#[inline]
fn copy_buffered(file: fs::File, writer: &mut impl Write) -> io::Result<()> {
    let mut reader = io::BufReader::with_capacity(IN_MEMORY_THRESHOLD, file);
    io::copy(&mut reader, writer)?;
    Ok(())
}

#[inline]
pub(crate) fn write_from_path(writer: &mut impl Write, path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    let mut file = fs::File::open(path)?;
    let file_size = file
        .metadata()
        .ok()
        .and_then(|meta| usize::try_from(meta.len()).ok());
    if let Some(size) = file_size {
        if size < IN_MEMORY_THRESHOLD {
            // NOTE: Use read_exact with pre-sized buffer to avoid fstat and dynamic allocation
            let mut contents = vec![0u8; size];
            file.read_exact(&mut contents)?;
            writer.write_all(&contents)?;
            return Ok(());
        }
        #[cfg(feature = "memmap")]
        {
            let mmap = utils::mmap::Mmap::map_with_size(file, size)?;
            writer.write_all(&mmap[..])?;
            return Ok(());
        }
    }
    // Fallback for large files without memmap, or when size is unknown
    copy_buffered(file, writer)
}

pub(crate) fn create_entry(
    item: &CollectedEntry,
    CreateOptions {
        option,
        keep_options,
        pathname_editor,
    }: &CreateOptions,
) -> io::Result<Option<NormalEntry>> {
    let CollectedEntry {
        path,
        store_as,
        metadata,
    } = item;
    let Some(entry_name) = pathname_editor.edit_entry_name(path) else {
        return Ok(None);
    };
    match store_as {
        StoreAs::Hardlink(source) => {
            let Some(reference) = pathname_editor.edit_hardlink(source) else {
                return Ok(None);
            };
            let entry = EntryBuilder::new_hard_link(entry_name, reference)?;
            apply_metadata(entry, path, keep_options, metadata)?.build()
        }
        StoreAs::Symlink => {
            let source = fs::read_link(path)?;
            let reference = pathname_editor.edit_symlink(&source);
            let entry = EntryBuilder::new_symlink(entry_name, reference)?;
            apply_metadata(entry, path, keep_options, metadata)?.build()
        }
        StoreAs::File => {
            let mut entry = EntryBuilder::new_file(entry_name, option)?;
            write_from_path(&mut entry, path)?;
            apply_metadata(entry, path, keep_options, metadata)?.build()
        }
        StoreAs::Dir => {
            let entry = EntryBuilder::new_dir(entry_name);
            apply_metadata(entry, path, keep_options, metadata)?.build()
        }
    }
    .map(Some)
}

pub(crate) fn entry_option(
    compression: CompressionAlgorithmArgs,
    cipher: CipherAlgorithmArgs,
    hash: HashAlgorithmArgs,
    password: Option<&[u8]>,
) -> WriteOptions {
    let (algorithm, level) = compression.algorithm();
    let mut option_builder = WriteOptions::builder();
    option_builder
        .compression(algorithm)
        .compression_level(level.unwrap_or_default())
        .encryption(if password.is_some() {
            cipher.algorithm()
        } else {
            pna::Encryption::No
        })
        .cipher_mode(cipher.mode())
        .hash_algorithm(hash.algorithm())
        .password(password);
    option_builder.build()
}

#[cfg_attr(target_os = "wasi", allow(unused_variables))]
pub(crate) fn apply_metadata(
    mut entry: EntryBuilder,
    path: &Path,
    keep_options: &KeepOptions,
    meta: &fs::Metadata,
) -> io::Result<EntryBuilder> {
    if let TimestampStrategy::Preserve {
        mtime,
        ctime,
        atime,
    } = keep_options.timestamp_strategy
    {
        if let Some(c) = ctime.resolve(meta.created().ok()) {
            entry.created_time(c);
        }
        if let Some(m) = mtime.resolve(meta.modified().ok()) {
            entry.modified_time(m);
        }
        if let Some(a) = atime.resolve(meta.accessed().ok()) {
            entry.accessed_time(a);
        }
    }
    #[cfg(unix)]
    if let OwnerStrategy::Preserve { options } = &keep_options.owner_strategy {
        use crate::utils::fs::{Group, User};
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let mode = meta.permissions().mode() as u16;
        // Get owner info: use overrides from OwnerStrategy if Preserve, else use filesystem values
        let uid = options.uid.unwrap_or(meta.uid());
        let gid = options.gid.unwrap_or(meta.gid());
        let uname = match &options.uname {
            None => User::from_uid(uid.into())?
                .name()
                .unwrap_or_default()
                .into(),
            Some(uname) => uname.clone(),
        };
        let gname = match &options.gname {
            None => Group::from_gid(gid.into())?
                .name()
                .unwrap_or_default()
                .into(),
            Some(gname) => gname.clone(),
        };
        entry.permission(pna::Permission::new(
            uid.into(),
            uname,
            gid.into(),
            gname,
            mode,
        ));
    }
    #[cfg(windows)]
    if let OwnerStrategy::Preserve { options } = &keep_options.owner_strategy {
        use crate::utils::os::windows::{fs::stat, security::SecurityDescriptor};

        let sd = SecurityDescriptor::try_from(path)?;
        let stat = stat(sd.path.as_ptr() as _)?;
        let mode = stat.st_mode;
        let user = sd.owner_sid()?;
        let group = sd.group_sid()?;
        // Get owner info: use overrides from OwnerStrategy
        let uid = options.uid.map_or(u64::MAX, Into::into);
        let gid = options.gid.map_or(u64::MAX, Into::into);
        let uname = options.uname.clone().unwrap_or(user.name);
        let gname = options.gname.clone().unwrap_or(group.name);
        entry.permission(pna::Permission::new(uid, uname, gid, gname, mode));
    }
    // On macOS, when mac_metadata_strategy is Always, AppleDouble packing via copyfile()
    // already includes xattrs and ACLs. Skip separate handling to avoid duplication.
    #[cfg(target_os = "macos")]
    let skip_xattr_acl = matches!(
        keep_options.mac_metadata_strategy,
        MacMetadataStrategy::Always
    );
    #[cfg(not(target_os = "macos"))]
    let skip_xattr_acl = false;

    #[cfg(feature = "acl")]
    if !skip_xattr_acl {
        #[cfg(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "macos",
            windows
        ))]
        if let AclStrategy::Always = keep_options.acl_strategy {
            use crate::chunk;
            use pna::RawChunk;
            let acl = utils::acl::get_facl(path)?;
            entry.add_extra_chunk(RawChunk::from_data(chunk::faCl, acl.platform.to_bytes()));
            for ace in acl.entries {
                entry.add_extra_chunk(RawChunk::from_data(chunk::faCe, ace.to_bytes()));
            }
        }
        #[cfg(not(any(
            target_os = "linux",
            target_os = "freebsd",
            target_os = "macos",
            windows
        )))]
        if let AclStrategy::Always = keep_options.acl_strategy {
            log::warn!("Currently acl is not supported on this platform.");
        }
    }
    #[cfg(not(feature = "acl"))]
    if let AclStrategy::Always = keep_options.acl_strategy {
        log::warn!("Please enable `acl` feature and rebuild and install pna.");
    }
    #[cfg(unix)]
    if !skip_xattr_acl {
        if let XattrStrategy::Always = keep_options.xattr_strategy {
            match utils::os::unix::fs::xattrs::get_xattrs(path) {
                Ok(xattrs) => {
                    for attr in xattrs {
                        entry.add_xattr(attr);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Unsupported => {
                    log::warn!(
                        "Extended attributes are not supported on filesystem for '{}': {}",
                        path.display(),
                        e
                    );
                }
                Err(e) => return Err(e),
            }
        }
    }
    #[cfg(not(unix))]
    if let XattrStrategy::Always = keep_options.xattr_strategy {
        log::warn!("Currently extended attribute is not supported on this platform.");
    }
    if let FflagsStrategy::Always = keep_options.fflags_strategy {
        match utils::fs::get_flags(path) {
            Ok(flags) => {
                for flag in flags {
                    entry.add_extra_chunk(crate::chunk::fflag_chunk(&flag));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Unsupported => {
                log::warn!(
                    "File flags are not supported on filesystem for '{}': {}",
                    path.display(),
                    e
                );
            }
            Err(e) => return Err(e),
        }
    }
    // macOS metadata (AppleDouble) - packs xattrs, ACLs, resource forks via copyfile()
    #[cfg(target_os = "macos")]
    if let MacMetadataStrategy::Always = keep_options.mac_metadata_strategy {
        use pna::RawChunk;
        match utils::os::unix::fs::copyfile::pack_apple_double(path) {
            Ok(apple_double_data) => {
                if !apple_double_data.is_empty() {
                    let len = apple_double_data.len();
                    entry.add_extra_chunk(RawChunk::from_data(
                        crate::chunk::maMd,
                        apple_double_data,
                    ));
                    log::debug!(
                        "Packed macOS metadata for '{}' ({len} bytes)",
                        path.display(),
                    );
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File has no Mac metadata, this is fine
            }
            Err(e) => {
                log::warn!(
                    "Failed to pack macOS metadata for '{}': {}",
                    path.display(),
                    e
                );
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    if let MacMetadataStrategy::Always = keep_options.mac_metadata_strategy {
        log::warn!("macOS metadata (--mac-metadata) is only supported on macOS.");
    }
    Ok(entry)
}

pub(crate) fn split_to_parts(
    mut entry_part: EntryPart<&[u8]>,
    first: usize,
    max: usize,
) -> io::Result<Vec<EntryPart<&[u8]>>> {
    let mut parts = vec![];
    let mut split_size = first;
    loop {
        match entry_part.try_split(split_size) {
            Ok((write_part, Some(remaining_part))) => {
                parts.push(write_part);
                entry_part = remaining_part;
                split_size = max;
            }
            Ok((write_part, None)) => {
                parts.push(write_part);
                break;
            }
            Err(unsplit_part) => {
                if split_size < max && parts.is_empty() {
                    // The entry's first chunk doesn't fit in remaining space (`first`),
                    // but it might fit in a fresh archive with full capacity (`max`).
                    // Retry with max size - the caller will handle creating a new archive
                    // when it sees the returned part exceeds remaining space.
                    entry_part = unsplit_part;
                    split_size = max;
                    continue;
                }
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "A chunk was detected that could not be divided into chunks smaller than the given size {max}"
                    ),
                ));
            }
        }
    }
    Ok(parts)
}

pub(crate) trait TransformStrategy {
    fn transform<W, T, F>(
        archive: &mut Archive<W>,
        password: Option<&[u8]>,
        read_entry: io::Result<ReadEntry<T>>,
        transformer: F,
    ) -> io::Result<()>
    where
        W: Write,
        T: AsRef<[u8]>,
        F: FnMut(io::Result<NormalEntry<T>>) -> io::Result<Option<NormalEntry<T>>>,
        NormalEntry<T>: From<NormalEntry>,
        NormalEntry<T>: Entry;
}

pub(crate) struct TransformStrategyUnSolid;

impl TransformStrategy for TransformStrategyUnSolid {
    fn transform<W, T, F>(
        archive: &mut Archive<W>,
        password: Option<&[u8]>,
        read_entry: io::Result<ReadEntry<T>>,
        mut transformer: F,
    ) -> io::Result<()>
    where
        W: Write,
        T: AsRef<[u8]>,
        F: FnMut(io::Result<NormalEntry<T>>) -> io::Result<Option<NormalEntry<T>>>,
        NormalEntry<T>: From<NormalEntry>,
        NormalEntry<T>: Entry,
    {
        match read_entry? {
            ReadEntry::Solid(s) => {
                for n in s.entries(password)? {
                    if let Some(entry) = transformer(n.map(Into::into))? {
                        archive.add_entry(entry)?;
                    }
                }
                Ok(())
            }
            ReadEntry::Normal(n) => {
                if let Some(entry) = transformer(Ok(n))? {
                    archive.add_entry(entry)?;
                }
                Ok(())
            }
        }
    }
}

pub(crate) struct TransformStrategyKeepSolid;

impl TransformStrategy for TransformStrategyKeepSolid {
    fn transform<W, T, F>(
        archive: &mut Archive<W>,
        password: Option<&[u8]>,
        read_entry: io::Result<ReadEntry<T>>,
        mut transformer: F,
    ) -> io::Result<()>
    where
        W: Write,
        T: AsRef<[u8]>,
        F: FnMut(io::Result<NormalEntry<T>>) -> io::Result<Option<NormalEntry<T>>>,
        NormalEntry<T>: From<NormalEntry>,
        NormalEntry<T>: Entry,
    {
        match read_entry? {
            ReadEntry::Solid(s) => {
                let header = s.header();
                let mut builder = SolidEntryBuilder::new(
                    WriteOptions::builder()
                        .compression(header.compression())
                        .encryption(header.encryption())
                        .cipher_mode(header.cipher_mode())
                        .password(password)
                        .build(),
                )?;
                for n in s.entries(password)? {
                    if let Some(entry) = transformer(n.map(Into::into))? {
                        builder.add_entry(entry)?;
                    }
                }
                archive.add_entry(builder.build()?)?;
                Ok(())
            }
            ReadEntry::Normal(n) => {
                if let Some(entry) = transformer(Ok(n))? {
                    archive.add_entry(entry)?;
                }
                Ok(())
            }
        }
    }
}

pub(crate) fn run_across_archive<R, F>(
    provider: impl IntoIterator<Item = R>,
    mut processor: F,
) -> io::Result<()>
where
    R: Read,
    F: FnMut(&mut Archive<R>) -> io::Result<()>,
{
    let mut iter = provider.into_iter();
    let mut archive = Archive::read_header(iter.next().expect(""))?;
    loop {
        processor(&mut archive)?;
        if archive.has_next_archive() {
            let next_reader = iter.next().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Archive is split, but no subsequent archives are found",
                )
            })?;
            archive = archive.read_next_archive(next_reader)?;
        } else {
            break;
        }
    }
    Ok(())
}

pub(crate) fn run_process_archive<'p, Provider, F>(
    archive_provider: impl IntoIterator<Item = impl Read>,
    mut password_provider: Provider,
    mut processor: F,
) -> io::Result<()>
where
    Provider: FnMut() -> Option<&'p [u8]>,
    F: FnMut(io::Result<NormalEntry>) -> io::Result<()>,
{
    let password = password_provider();
    run_read_entries(archive_provider, |entry| match entry? {
        ReadEntry::Solid(solid) => solid.entries(password)?.try_for_each(&mut processor),
        ReadEntry::Normal(regular) => processor(Ok(regular)),
    })
}

#[cfg(feature = "memmap")]
pub(crate) fn run_across_archive_mem<'d, F>(
    archives: impl IntoIterator<Item = &'d [u8]>,
    mut processor: F,
) -> io::Result<()>
where
    F: FnMut(&mut Archive<&'d [u8]>) -> io::Result<()>,
{
    let mut iter = archives.into_iter();
    let mut archive = Archive::read_header_from_slice(iter.next().expect(""))?;

    loop {
        processor(&mut archive)?;
        if archive.has_next_archive() {
            let next_reader = iter.next().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Archive is split, but no subsequent archives are found",
                )
            })?;
            archive = archive.read_next_archive_from_slice(next_reader)?;
        } else {
            break;
        }
    }
    Ok(())
}

#[cfg(feature = "memmap")]
pub(crate) fn run_read_entries_mem<'d, F>(
    archives: impl IntoIterator<Item = &'d [u8]>,
    mut processor: F,
) -> io::Result<()>
where
    F: FnMut(io::Result<ReadEntry<Cow<'d, [u8]>>>) -> io::Result<()>,
{
    run_across_archive_mem(archives, |archive| {
        archive.entries_slice().try_for_each(&mut processor)
    })
}

#[cfg(feature = "memmap")]
pub(crate) fn run_entries<'d, 'p, Provider, F>(
    archives: impl IntoIterator<Item = &'d [u8]>,
    mut password_provider: Provider,
    mut processor: F,
) -> io::Result<()>
where
    Provider: FnMut() -> Option<&'p [u8]>,
    F: FnMut(io::Result<NormalEntry<Cow<'d, [u8]>>>) -> io::Result<()>,
{
    let password = password_provider();
    run_read_entries_mem(archives, |entry| match entry? {
        ReadEntry::Solid(s) => s
            .entries(password)?
            .try_for_each(|r| processor(r.map(Into::into))),
        ReadEntry::Normal(r) => processor(Ok(r)),
    })
}

#[cfg(feature = "memmap")]
pub(crate) fn run_transform_entry<'d, 'p, W, Provider, F, Transform>(
    writer: W,
    archives: impl IntoIterator<Item = &'d [u8]>,
    mut password_provider: Provider,
    mut processor: F,
    _strategy: Transform,
) -> anyhow::Result<()>
where
    W: Write,
    Provider: FnMut() -> Option<&'p [u8]>,
    F: FnMut(
        io::Result<NormalEntry<Cow<'d, [u8]>>>,
    ) -> io::Result<Option<NormalEntry<Cow<'d, [u8]>>>>,
    Transform: TransformStrategy,
{
    let password = password_provider();
    let mut out_archive = Archive::write_header(writer)?;
    run_read_entries_mem(archives, |entry| {
        Transform::transform(&mut out_archive, password, entry, &mut processor)
    })?;
    out_archive.finalize()?;
    Ok(())
}

pub(crate) fn run_read_entries<F>(
    archive_provider: impl IntoIterator<Item = impl Read>,
    mut processor: F,
) -> io::Result<()>
where
    F: FnMut(io::Result<ReadEntry>) -> io::Result<()>,
{
    run_across_archive(archive_provider, |archive| {
        archive.entries().try_for_each(&mut processor)
    })
}

#[cfg(not(feature = "memmap"))]
pub(crate) fn run_transform_entry<'p, W, Provider, F, Transform>(
    writer: W,
    archives: impl IntoIterator<Item = impl Read>,
    mut password_provider: Provider,
    mut processor: F,
    _strategy: Transform,
) -> anyhow::Result<()>
where
    W: Write,
    Provider: FnMut() -> Option<&'p [u8]>,
    F: FnMut(io::Result<NormalEntry>) -> io::Result<Option<NormalEntry>>,
    Transform: TransformStrategy,
{
    let password = password_provider();
    let mut out_archive = Archive::write_header(writer)?;
    run_read_entries(archives, |entry| {
        Transform::transform(&mut out_archive, password, entry, &mut processor)
    })?;
    out_archive.finalize()?;
    Ok(())
}

#[cfg(not(feature = "memmap"))]
pub(crate) fn run_entries<'p, Provider, F>(
    archives: Vec<fs::File>,
    password_provider: Provider,
    processor: F,
) -> io::Result<()>
where
    Provider: FnMut() -> Option<&'p [u8]>,
    F: FnMut(io::Result<NormalEntry>) -> io::Result<()>,
{
    run_process_archive(archives, password_provider, processor)
}

pub(crate) fn write_split_archive(
    archive: impl AsRef<Path>,
    entries: impl Iterator<Item = io::Result<impl Entry + Sized>>,
    max_file_size: usize,
    overwrite: bool,
) -> anyhow::Result<()> {
    write_split_archive_path(
        archive,
        entries,
        |base, n| base.with_part(n),
        max_file_size,
        overwrite,
    )
}

pub(crate) fn write_split_archive_path<F, P>(
    archive: impl AsRef<Path>,
    entries: impl Iterator<Item = io::Result<impl Entry + Sized>>,
    mut get_part_path: F,
    max_file_size: usize,
    overwrite: bool,
) -> anyhow::Result<()>
where
    F: FnMut(&Path, usize) -> P,
    P: AsRef<Path>,
{
    let archive = archive.as_ref();
    let first_item_path = get_part_path(archive, 1);
    let first_item_path = first_item_path.as_ref();
    let file = utils::fs::file_create(first_item_path, overwrite)?;
    let buffered = io::BufWriter::with_capacity(64 * 1024, file);
    write_split_archive_writer(
        buffered,
        entries,
        |n| {
            let file = utils::fs::file_create(get_part_path(archive, n), overwrite)?;
            Ok(io::BufWriter::with_capacity(64 * 1024, file))
        },
        max_file_size,
        |n| {
            if n == 1 {
                fs::rename(first_item_path, archive)?;
            };
            Ok(())
        },
    )
}

pub(crate) fn write_split_archive_writer<W, F, C>(
    initial_writer: W,
    entries: impl Iterator<Item = io::Result<impl Entry + Sized>>,
    mut get_next_writer: F,
    max_file_size: usize,
    mut on_complete: C,
) -> anyhow::Result<()>
where
    W: Write,
    F: FnMut(usize) -> io::Result<W>,
    C: FnMut(usize) -> io::Result<()>,
{
    if max_file_size < MIN_SPLIT_PART_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Split size must be at least {MIN_SPLIT_PART_BYTES} bytes to accommodate headers"
            ),
        )
        .into());
    }
    let mut part_num = 1;
    let mut writer = Archive::write_header(initial_writer)?;

    // NOTE: max_file_size - (PNA_HEADER + AHED + ANXT + AEND)
    let max_file_size = max_file_size - SPLIT_ARCHIVE_OVERHEAD_BYTES;
    let mut written_entry_size = 0;
    for entry in entries {
        let p = EntryPart::from(entry?);
        let parts = split_to_parts(
            p.as_ref(),
            max_file_size - written_entry_size,
            max_file_size,
        )?;
        for part in parts {
            if written_entry_size + part.bytes_len() > max_file_size {
                part_num += 1;
                let file = get_next_writer(part_num)?;
                writer = writer.split_to_next_archive(file)?;
                written_entry_size = 0;
            }
            written_entry_size += writer.add_entry_part(part)?;
        }
    }
    writer.finalize()?;
    on_complete(part_num)?;
    Ok(())
}

#[inline]
fn read_paths_reader(reader: impl BufRead, nul: bool) -> io::Result<Vec<String>> {
    if nul {
        utils::io::read_to_nul(reader)
    } else {
        utils::io::read_to_lines(reader)
    }
}

#[inline]
pub(crate) fn read_paths<P: AsRef<Path>>(path: P, nul: bool) -> io::Result<Vec<String>> {
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    read_paths_reader(reader, nul)
}

#[inline]
pub(crate) fn read_paths_stdin(nul: bool) -> io::Result<Vec<String>> {
    read_paths_reader(io::stdin().lock(), nul)
}

/// Apply chroot to the current directory if enabled.
///
/// On Unix systems (excluding Fuchsia), this changes the root directory to the
/// current directory and resets the working directory to `/`.
/// On other platforms, a warning is emitted since chroot is unsupported.
#[allow(unused_variables)]
pub(crate) fn apply_chroot(chroot: bool) -> anyhow::Result<()> {
    if !chroot {
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "fuchsia")))]
    {
        std::os::unix::fs::chroot(
            std::env::current_dir().with_context(|| "resolving current directory before chroot")?,
        )
        .with_context(|| "chroot into current directory")?;
        std::env::set_current_dir("/").with_context(|| "changing directory to / after chroot")?;
    }
    #[cfg(not(all(unix, not(target_os = "fuchsia"))))]
    {
        log::warn!("chroot not supported on this platform");
    }
    Ok(())
}

/// Transforms entries from a source archive, applying path and ownership transformations.
///
/// This function reads entries from the source archive and yields transformed entries
/// that can be added to a target archive. For solid entries, they are expanded to
/// individual normal entries. Encrypted solid entries can be decrypted and expanded
/// if a password is provided; without a password they will cause an error.
///
/// The entry data (FDAT chunks) is preserved as-is, maintaining the original
/// compression and encryption. Only the entry headers (paths, ownership) are modified.
///
/// Entries whose path matches the filter exclusion rules will be skipped.
/// Time filters are also applied to filter entries by timestamps.
pub(crate) fn transform_archive_entries<R: io::Read>(
    reader: R,
    create_options: &CreateOptions,
    filter: &PathFilter<'_>,
    time_filters: &TimeFilters,
    password: Option<&[u8]>,
) -> io::Result<Vec<io::Result<Option<NormalEntry>>>> {
    let mut archive = Archive::read_header(reader)?;
    let mut results = Vec::new();

    for entry_result in archive.entries().extract_solid_entries(password) {
        match entry_result {
            Ok(entry) => {
                if filter.excluded(entry.header().path()) {
                    continue;
                }
                let ctime = entry.metadata().created_time();
                let mtime = entry.metadata().modified_time();
                if !time_filters.matches_or_inactive(ctime, mtime) {
                    continue;
                }
                results.push(transform_normal_entry(entry, create_options));
            }
            Err(e) => results.push(Err(e)),
        }
    }

    Ok(results)
}

/// Reads entries from an archive source (file or stdin) and transforms them.
pub(crate) fn read_archive_source(
    source: &ArchiveSource,
    create_options: &CreateOptions,
    filter: &PathFilter<'_>,
    time_filters: &TimeFilters,
    password: Option<&[u8]>,
) -> io::Result<Vec<io::Result<Option<NormalEntry>>>> {
    match source {
        ArchiveSource::File(path) => {
            let file = fs::File::open(path)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", path.display(), e)))?;
            let reader = io::BufReader::with_capacity(64 * 1024, file);
            transform_archive_entries(reader, create_options, filter, time_filters, password)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", path.display(), e)))
        }
        ArchiveSource::Stdin => {
            let reader = io::BufReader::new(io::stdin().lock());
            transform_archive_entries(reader, create_options, filter, time_filters, password)
                .map_err(|e| io::Error::new(e.kind(), format!("<stdin>: {}", e)))
        }
    }
}

/// Transforms a single normal entry, applying path and ownership modifications.
fn transform_normal_entry(
    entry: NormalEntry,
    CreateOptions {
        pathname_editor,
        keep_options,
        ..
    }: &CreateOptions,
) -> io::Result<Option<NormalEntry>> {
    // Apply path transformation
    let original_name = entry.header().path();
    let Some(new_name) = pathname_editor.edit_entry_name(original_name.as_ref()) else {
        // Entry path was stripped away entirely
        return Ok(None);
    };

    let mut result = entry.with_name(new_name);

    // Apply ownership overrides from owner_strategy
    if let OwnerStrategy::Preserve {
        options:
            OwnerOptions {
                uid,
                gid,
                uname,
                gname,
            },
    } = &keep_options.owner_strategy
    {
        // Only apply if at least one override is specified
        if uid.is_some() || gid.is_some() || uname.is_some() || gname.is_some() {
            if let Some(perm) = result.metadata().permission() {
                let new_perm = pna::Permission::new(
                    uid.map(u64::from).unwrap_or_else(|| perm.uid()),
                    uname.clone().unwrap_or_else(|| perm.uname().to_string()),
                    gid.map(u64::from).unwrap_or_else(|| perm.gid()),
                    gname.clone().unwrap_or_else(|| perm.gname().to_string()),
                    perm.permissions(),
                );
                let metadata = result.metadata().clone().with_permission(Some(new_perm));
                result = result.with_metadata(metadata);
            }
        }
    }

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const EMPTY_PATTERNS: [&str; 0] = [];

    fn empty_path_filter<'a>() -> PathFilter<'a> {
        PathFilter::new(EMPTY_PATTERNS, EMPTY_PATTERNS)
    }

    fn empty_time_filters() -> TimeFilters {
        TimeFilters {
            ctime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
            mtime: TimeFilter {
                newer_than: None,
                older_than: None,
            },
        }
    }

    fn default_collect_options<'a>(
        filter: &'a PathFilter<'a>,
        time_filters: &'a TimeFilters,
    ) -> CollectOptions<'a> {
        CollectOptions {
            recursive: false,
            keep_dir: false,
            gitignore: false,
            nodump: false,
            follow_links: false,
            follow_command_links: false,
            one_file_system: false,
            filter,
            time_filters,
        }
    }

    #[test]
    fn collect_items_only_file() {
        let source = concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/raw",);
        let filter = empty_path_filter();
        let time_filters = empty_time_filters();
        let options = default_collect_options(&filter, &time_filters);
        let mut resolver = HardlinkResolver::new(options.follow_links);
        let items = collect_items_from_paths([source], &options, &mut resolver).unwrap();
        assert_eq!(
            items.into_iter().map(|it| it.path).collect::<HashSet<_>>(),
            HashSet::new()
        );
    }

    #[test]
    fn collect_items_keep_dir() {
        let source = concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/raw",);
        let filter = empty_path_filter();
        let time_filters = empty_time_filters();
        let mut options = default_collect_options(&filter, &time_filters);
        options.keep_dir = true;
        let mut resolver = HardlinkResolver::new(options.follow_links);
        let items = collect_items_from_paths([source], &options, &mut resolver).unwrap();
        assert_eq!(
            items.into_iter().map(|it| it.path).collect::<HashSet<_>>(),
            [concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../resources/test/raw",
            )]
            .into_iter()
            .map(Into::into)
            .collect::<HashSet<_>>()
        );
    }

    #[test]
    fn collect_items_recursive() {
        let source = concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/test/raw",);
        let filter = empty_path_filter();
        let time_filters = empty_time_filters();
        let mut options = default_collect_options(&filter, &time_filters);
        options.recursive = true;
        let mut resolver = HardlinkResolver::new(options.follow_links);
        let items = collect_items_from_paths([source], &options, &mut resolver).unwrap();
        assert_eq!(
            items.into_iter().map(|it| it.path).collect::<HashSet<_>>(),
            [
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/first/second/third/pna.txt"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/images/icon.bmp"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/images/icon.png"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/images/icon.svg"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/parent/child.txt"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/pna/empty.pna"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/pna/nest.pna"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/empty.txt"
                ),
                concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../resources/test/raw/text.txt"
                ),
            ]
            .into_iter()
            .map(Into::into)
            .collect::<HashSet<_>>()
        );
    }

    mod item_source_parse {
        use super::*;

        #[test]
        fn at_alone_is_stdin() {
            let result = ItemSource::parse("@");
            assert!(matches!(result, ItemSource::Archive(ArchiveSource::Stdin)));
        }

        #[test]
        fn at_dash_is_stdin() {
            let result = ItemSource::parse("@-");
            assert!(matches!(result, ItemSource::Archive(ArchiveSource::Stdin)));
        }

        #[test]
        fn at_path_is_archive_file() {
            let result = ItemSource::parse("@archive.pna");
            assert!(matches!(
                result,
                ItemSource::Archive(ArchiveSource::File(p)) if p == Path::new("archive.pna")
            ));
        }

        #[test]
        fn plain_path_is_filesystem() {
            let result = ItemSource::parse("some/path");
            assert!(matches!(
                result,
                ItemSource::Filesystem(p) if p == Path::new("some/path")
            ));
        }

        #[test]
        fn dot_slash_at_is_filesystem_escape() {
            // Following bsdtar convention: ./@file escapes the @ prefix
            let result = ItemSource::parse("./@file");
            assert!(matches!(
                result,
                ItemSource::Filesystem(p) if p == Path::new("./@file")
            ));
        }

        #[test]
        fn parse_many_mixed() {
            let args = vec![
                "file1".to_string(),
                "@archive.pna".to_string(),
                "@".to_string(),
                "./@literal".to_string(),
            ];
            let results = ItemSource::parse_many(&args);
            assert_eq!(results.len(), 4);
            assert!(matches!(&results[0], ItemSource::Filesystem(p) if p == Path::new("file1")));
            assert!(
                matches!(&results[1], ItemSource::Archive(ArchiveSource::File(p)) if p == Path::new("archive.pna"))
            );
            assert!(matches!(
                &results[2],
                ItemSource::Archive(ArchiveSource::Stdin)
            ));
            assert!(
                matches!(&results[3], ItemSource::Filesystem(p) if p == Path::new("./@literal"))
            );
        }

        #[test]
        fn validate_no_duplicate_stdin_ok() {
            let sources = vec![
                ItemSource::Filesystem(PathBuf::from("file")),
                ItemSource::Archive(ArchiveSource::Stdin),
                ItemSource::Archive(ArchiveSource::File(PathBuf::from("archive.pna"))),
            ];
            assert!(super::validate_no_duplicate_stdin(&sources).is_ok());
        }

        #[test]
        fn validate_no_duplicate_stdin_error() {
            let sources = vec![
                ItemSource::Archive(ArchiveSource::Stdin),
                ItemSource::Archive(ArchiveSource::Stdin),
            ];
            assert!(super::validate_no_duplicate_stdin(&sources).is_err());
        }

        #[test]
        fn validate_no_duplicate_stdin_empty() {
            let sources: Vec<ItemSource> = vec![];
            assert!(super::validate_no_duplicate_stdin(&sources).is_ok());
        }
    }
}
