use crate::{
    cli::{ArchiveFileArgs, FileOperands, PasswordArgs},
    command::{
        Command, ExitCodeError, ask_password,
        core::{SplitArchiveReader, cmp_at_stored_precision, collect_split_archives},
    },
    utils::{BsdGlobMatcher, io::streams_equal},
};
use bitflags::bitflags;
use clap::{Parser, ValueEnum};
use pna::prelude::SystemTimeDurationExt;
use pna::{DataKind, EntryContent, NormalEntry, ReadOptions};
use same_file::is_same_file;
use std::cmp::Ordering;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::time::SystemTime;
use std::{collections::BTreeMap, fmt, fs, io, path::Path};

#[derive(Parser, Clone, Debug)]
pub(crate) struct DiffCommand {
    #[command(flatten)]
    archive: ArchiveFileArgs,
    #[command(flatten)]
    files: FileOperands,
    #[command(flatten)]
    password: PasswordArgs,
    #[arg(
        long,
        conflicts_with = "compare",
        help = "Compare directory mtime and ownership (by default, only mode is compared for directories)"
    )]
    full_compare: bool,
    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        requires = "unstable",
        help_heading = "Unstable Options",
        help = "Compare only selected fields; repeat or separate values with commas",
        long_help = "Compare only selected fields; repeat or separate values with commas. `default` stands for the fields compared when this option is omitted, and combines with any other value rather than replacing it. Missing paths and file type mismatches are reported regardless of the selection. Naming a field that cannot be compared - unsupported on this platform, or no value recorded in the archive - reports the differences it did find and then exits 2, so a run with both differences and uncomparable fields is indistinguishable from an uncomparable-only run by exit code alone. Conflicts with --full-compare."
    )]
    compare: Vec<CompareFieldArg>,
    #[arg(
        long,
        default_value = "plain",
        help = "Output format [unstable: jsonl]",
        long_help = "Output format. plain: GNU tar --diff style text. jsonl: one JSON Lines record per difference with fields `path`, `kind` (one of: missing, size, content, mode, mtime, uid, gid, type, symlink, hardlink) and, for kind=hardlink only, `target` (the stored link target). mode/uid/gid comparisons only occur on Unix."
    )]
    format: Format,
}

/// A value accepted by `--compare`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
#[value(rename_all = "lower")]
enum CompareFieldArg {
    /// Not a field; adds the default fields.
    Default,
    Size,
    Content,
    Mtime,
    Mode,
    Uid,
    Gid,
    Symlink,
    Hardlink,
}

impl CompareFieldArg {
    const fn field(self) -> Option<CompareField> {
        match self {
            Self::Default => None,
            Self::Size => Some(CompareField::Size),
            Self::Content => Some(CompareField::Content),
            Self::Mtime => Some(CompareField::Mtime),
            Self::Mode => Some(CompareField::Mode),
            Self::Uid => Some(CompareField::Uid),
            Self::Gid => Some(CompareField::Gid),
            Self::Symlink => Some(CompareField::Symlink),
            Self::Hardlink => Some(CompareField::Hardlink),
        }
    }
}

/// An aspect of an entry that can be compared against the filesystem.
///
/// Each discriminant is the single bit this field occupies in [`CompareFields`].
/// `ValueEnum` is derived only for its spelling: diagnostics name a field the
/// way `--compare` accepts it.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, ValueEnum)]
#[value(rename_all = "lower")]
#[repr(u8)]
enum CompareField {
    Size = 1 << 0,
    Content = 1 << 1,
    Mtime = 1 << 2,
    Mode = 1 << 3,
    Uid = 1 << 4,
    Gid = 1 << 5,
    Symlink = 1 << 6,
    Hardlink = 1 << 7,
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    struct CompareFields: u8 {
        const SIZE = CompareField::Size as u8;
        const CONTENT = CompareField::Content as u8;
        const MTIME = CompareField::Mtime as u8;
        const MODE = CompareField::Mode as u8;
        const UID = CompareField::Uid as u8;
        const GID = CompareField::Gid as u8;
        const SYMLINK = CompareField::Symlink as u8;
        const HARDLINK = CompareField::Hardlink as u8;
    }
}

impl CompareField {
    #[inline]
    const fn flag(self) -> CompareFields {
        CompareFields::from_bits_retain(self as u8)
    }

    #[inline]
    const fn is_supported(self) -> bool {
        cfg!(unix) || !matches!(self, Self::Mode | Self::Uid | Self::Gid)
    }
}

impl fmt::Display for CompareField {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_possible_value().unwrap().get_name())
    }
}

/// Why an explicitly requested field could not be compared.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Uncomparable {
    UnsupportedPlatform,
    NotRecorded,
    NotReported,
}

impl fmt::Display for Uncomparable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnsupportedPlatform => "unsupported on this platform",
            Self::NotRecorded => "not recorded in the archive",
            Self::NotReported => "not reported by the filesystem",
        })
    }
}

/// Explicitly requested fields that turned out not to be comparable.
///
/// A field the user named by hand is a demand, so failing to compare it must not
/// pass as "no difference". Fields reached through the default profile stay
/// best-effort: [`Self::record_entry`] ignores them.
#[derive(Debug)]
struct UncomparedFields {
    requested: CompareFields,
    /// Value is the first entry path that hit this and how many entries did.
    /// `None` for a platform-level failure, which is tied to no entry.
    seen: BTreeMap<(CompareField, Uncomparable), Option<(String, usize)>>,
}

impl UncomparedFields {
    fn new(requested: CompareFields) -> Self {
        Self {
            requested,
            seen: BTreeMap::new(),
        }
    }

    fn record_platform(&mut self, field: CompareField) {
        self.seen
            .insert((field, Uncomparable::UnsupportedPlatform), None);
    }

    fn record_entry(&mut self, field: CompareField, reason: Uncomparable, path: &str) {
        if !self.requested.contains(field.flag()) {
            return;
        }
        self.seen
            .entry((field, reason))
            .or_insert(None)
            .get_or_insert_with(|| (path.to_owned(), 0))
            .1 += 1;
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

impl fmt::Display for UncomparedFields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("cannot compare the requested field(s):")?;
        for ((field, reason), occurrences) in &self.seen {
            write!(f, "\n  {field}: {reason}")?;
            if let Some((path, count)) = occurrences {
                write!(f, " ({count} entries, e.g. {path})")?;
            }
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
#[value(rename_all = "lower")]
enum Format {
    Plain,
    JsonL,
}

impl Format {
    /// Returns true if this format is unstable and requires --unstable flag
    #[inline]
    const fn is_unstable(self) -> bool {
        matches!(self, Self::JsonL)
    }
}

impl fmt::Display for Format {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_possible_value().unwrap().get_name())
    }
}

impl Command for DiffCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        match diff_archive(ctx, self) {
            Ok(0) => Ok(()),
            Ok(_) => Err(ExitCodeError::silent(1).into()),
            Err(err) => Err(ExitCodeError::with_source(2, err).into()),
        }
    }
}

#[hooq::hooq(anyhow)]
fn diff_archive(ctx: &crate::cli::GlobalContext, args: DiffCommand) -> anyhow::Result<usize> {
    if args.format.is_unstable() && !ctx.unstable() {
        anyhow::bail!(
            "The '--format {}' option is unstable and requires --unstable flag",
            args.format
        );
    }
    let (options, mut uncompared) =
        CompareOptions::new(args.compare, args.full_compare, args.format);
    if options.compares_nothing() {
        // Nothing survives to compare, so fail before prompting for a password.
        anyhow::bail!("{uncompared}");
    }
    let password = ask_password(args.password)?;
    let archives = collect_split_archives(args.archive.require_file()?)?;

    let mut globs = BsdGlobMatcher::new(args.files.files.iter().map(|s| s.as_str()));
    let filter_enabled = !globs.is_empty();

    let read_options = ReadOptions::with_password(password.as_deref());
    let mut source = SplitArchiveReader::new(archives)?;
    let mut diff_count = 0usize;
    source.for_each_entry(
        &read_options,
        #[hooq::skip_all]
        |entry| {
            let entry = entry?;
            let path = entry.header().path();

            if filter_enabled && !globs.matches(path) {
                return Ok(());
            }

            diff_count += compare_entry(entry, &read_options, &options, &mut uncompared)?;
            Ok(())
        },
    )?;

    globs.ensure_all_matched()?;

    // Differences found are already on stdout, so failing here loses no output.
    if !uncompared.is_empty() {
        anyhow::bail!("{uncompared}");
    }

    Ok(diff_count)
}

/// Difference types detected during archive-filesystem comparison.
/// Message format follows GNU tar --diff for compatibility.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind")]
enum DiffKind {
    /// File/directory does not exist on filesystem
    #[serde(rename = "missing")]
    Missing,
    /// File size differs
    #[serde(rename = "size")]
    SizeDiffers,
    /// File contents differ (same size)
    #[serde(rename = "content")]
    ContentsDiffer,
    /// Permission mode differs
    #[cfg(unix)]
    #[serde(rename = "mode")]
    ModeDiffers,
    /// Modification time differs
    #[serde(rename = "mtime")]
    MtimeDiffers,
    /// User ID differs
    #[cfg(unix)]
    #[serde(rename = "uid")]
    UidDiffers,
    /// Group ID differs
    #[cfg(unix)]
    #[serde(rename = "gid")]
    GidDiffers,
    /// File type differs (e.g., file vs directory)
    #[serde(rename = "type")]
    TypeMismatch,
    /// Symbolic link target differs
    #[serde(rename = "symlink")]
    SymlinkDiffers,
    /// Hardlink relationship broken
    #[serde(rename = "hardlink")]
    NotLinked { target: String },
}

impl DiffKind {
    /// Returns a displayable message for this difference.
    fn display<'a>(&'a self, path: &'a str) -> DiffMessage<'a> {
        DiffMessage { kind: self, path }
    }
}

#[derive(serde::Serialize)]
struct DiffRecord<'a> {
    path: &'a str,
    #[serde(flatten)]
    kind: &'a DiffKind,
}

fn report(kind: &DiffKind, path: &str, format: Format) {
    match format {
        Format::Plain => println!("{}", kind.display(path)),
        Format::JsonL => println!(
            "{}",
            serde_json::to_string(&DiffRecord { path, kind }).unwrap()
        ),
    }
}

/// A tar-compatible difference message that implements `Display`.
struct DiffMessage<'a> {
    kind: &'a DiffKind,
    path: &'a str,
}

impl fmt::Display for DiffMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            DiffKind::Missing => {
                write!(
                    f,
                    "{}: Warning: Cannot stat: No such file or directory",
                    self.path
                )
            }
            DiffKind::SizeDiffers => write!(f, "{}: Size differs", self.path),
            DiffKind::ContentsDiffer => write!(f, "{}: Contents differ", self.path),
            #[cfg(unix)]
            DiffKind::ModeDiffers => write!(f, "{}: Mode differs", self.path),
            DiffKind::MtimeDiffers => write!(f, "{}: Mod time differs", self.path),
            #[cfg(unix)]
            DiffKind::UidDiffers => write!(f, "{}: Uid differs", self.path),
            #[cfg(unix)]
            DiffKind::GidDiffers => write!(f, "{}: Gid differs", self.path),
            DiffKind::TypeMismatch => write!(f, "{}: File type differs", self.path),
            DiffKind::SymlinkDiffers => write!(f, "{}: Symlink differs", self.path),
            DiffKind::NotLinked { target } => write!(f, "{}: Not linked to {target}", self.path),
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct CompareOptions {
    default_profile: bool,
    /// Fields named explicitly on the command line, with the fields unsupported
    /// on this platform already dropped.
    fields: CompareFields,
    full_compare: bool,
    format: Format,
}

impl CompareOptions {
    fn new(
        compare: Vec<CompareFieldArg>,
        full_compare: bool,
        format: Format,
    ) -> (Self, UncomparedFields) {
        let default_profile = compare.is_empty() || compare.contains(&CompareFieldArg::Default);
        let named = compare.iter().filter_map(|arg| arg.field());
        let fields = named
            .clone()
            .filter(|field| field.is_supported())
            .fold(CompareFields::empty(), |acc, field| acc | field.flag());
        let mut uncompared = UncomparedFields::new(fields);
        for field in named.filter(|field| !field.is_supported()) {
            uncompared.record_platform(field);
        }
        (
            Self {
                default_profile,
                fields,
                full_compare,
                format,
            },
            uncompared,
        )
    }

    #[inline]
    fn enabled(&self, field: CompareField, data_kind: DataKind) -> bool {
        self.fields.contains(field.flag()) || self.default_enabled(field, data_kind)
    }

    fn default_enabled(&self, field: CompareField, data_kind: DataKind) -> bool {
        if !self.default_profile || !field.is_supported() {
            return false;
        }
        match field {
            CompareField::Size | CompareField::Content => data_kind == DataKind::FILE,
            CompareField::Symlink => data_kind == DataKind::SYMBOLIC_LINK,
            CompareField::Hardlink => data_kind == DataKind::HARD_LINK,
            CompareField::Mode => matches!(data_kind, DataKind::FILE | DataKind::DIRECTORY),
            CompareField::Mtime | CompareField::Uid | CompareField::Gid => {
                data_kind == DataKind::FILE
                    || (data_kind == DataKind::DIRECTORY && self.full_compare)
            }
        }
    }

    /// True when every explicitly named field was dropped as unsupported on this
    /// platform, leaving nothing to compare.
    #[inline]
    fn compares_nothing(&self) -> bool {
        !self.default_profile && self.fields.is_empty()
    }
}

fn matches_at_stored_precision(archived: pna::Duration, fs: SystemTime) -> bool {
    fs.try_duration_since_unix_epoch_signed()
        .is_ok_and(|fs| cmp_at_stored_precision(archived, fs) == Ordering::Equal)
}

fn compare_metadata<T: AsRef<[u8]>>(
    entry: &NormalEntry<T>,
    fs_meta: &fs::Metadata,
    data_kind: DataKind,
    options: &CompareOptions,
    uncompared: &mut UncomparedFields,
) -> Vec<DiffKind> {
    let path = entry.header().path().as_str();
    let mut missing =
        |field: CompareField, reason: Uncomparable| uncompared.record_entry(field, reason, path);
    let mut diffs = Vec::new();
    // Resolving ownership allocates owner name and SID strings that are never
    // compared here, so only pay for it when an ownership field is enabled.
    #[cfg(unix)]
    let ownership = if options.enabled(CompareField::Mode, data_kind)
        || options.enabled(CompareField::Uid, data_kind)
        || options.enabled(CompareField::Gid, data_kind)
    {
        crate::ext::ResolvedOwnership::from_metadata(entry.metadata())
    } else {
        crate::ext::ResolvedOwnership::default()
    };

    #[cfg(unix)]
    if options.enabled(CompareField::Mode, data_kind) {
        match ownership.mode {
            Some(mode) => {
                let archive_mode = mode & 0o7777;
                let fs_mode = (fs_meta.permissions().mode() & 0o7777) as u16;
                if archive_mode != fs_mode {
                    diffs.push(DiffKind::ModeDiffers);
                }
            }
            None => missing(CompareField::Mode, Uncomparable::NotRecorded),
        }
    }

    if options.enabled(CompareField::Mtime, data_kind) {
        match entry.metadata().modified() {
            Some(archive_mtime) => match fs_meta.modified() {
                Ok(fs_mtime) => {
                    if !matches_at_stored_precision(archive_mtime, fs_mtime) {
                        diffs.push(DiffKind::MtimeDiffers);
                    }
                }
                Err(_) => missing(CompareField::Mtime, Uncomparable::NotReported),
            },
            None => missing(CompareField::Mtime, Uncomparable::NotRecorded),
        }
    }

    #[cfg(unix)]
    if options.enabled(CompareField::Uid, data_kind) {
        match ownership.uid {
            Some(uid) if uid != fs_meta.uid() as u64 => diffs.push(DiffKind::UidDiffers),
            Some(_) => {}
            None => missing(CompareField::Uid, Uncomparable::NotRecorded),
        }
    }

    #[cfg(unix)]
    if options.enabled(CompareField::Gid, data_kind) {
        match ownership.gid {
            Some(gid) if gid != fs_meta.gid() as u64 => diffs.push(DiffKind::GidDiffers),
            Some(_) => {}
            None => missing(CompareField::Gid, Uncomparable::NotRecorded),
        }
    }

    diffs
}

fn compare_entry<T: AsRef<[u8]>>(
    entry: NormalEntry<T>,
    read_options: &ReadOptions,
    options: &CompareOptions,
    uncompared: &mut UncomparedFields,
) -> io::Result<usize> {
    let data_kind = entry.header().data_kind();
    let path = entry.header().path();
    let path_str = path.as_str();
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            report(&DiffKind::Missing, path_str, options.format);
            return Ok(1);
        }
        Err(e) => return Err(e),
    };

    let type_matches = match data_kind {
        DataKind::FILE => meta.is_file(),
        DataKind::DIRECTORY => meta.is_dir(),
        DataKind::SYMBOLIC_LINK => meta.is_symlink(),
        DataKind::HARD_LINK => meta.is_file(),
        _ => false,
    };
    if !type_matches {
        report(&DiffKind::TypeMismatch, path_str, options.format);
        return Ok(1);
    }

    // Report metadata differences before touching the entry data, so that an I/O
    // failure while reading it cannot swallow what has already been found.
    let meta_diffs = compare_metadata(&entry, &meta, data_kind, options, uncompared);
    for diff in &meta_diffs {
        report(diff, path_str, options.format);
    }
    let mut diff_count = meta_diffs.len();

    if let Some(diff) = compare_data(
        &entry,
        &meta,
        path,
        data_kind,
        read_options,
        options,
        uncompared,
    )? {
        report(&diff, path_str, options.format);
        diff_count += 1;
    }
    Ok(diff_count)
}

fn compare_data<T: AsRef<[u8]>>(
    entry: &NormalEntry<T>,
    fs_meta: &fs::Metadata,
    path: &pna::EntryName,
    data_kind: DataKind,
    read_options: &ReadOptions,
    options: &CompareOptions,
    uncompared: &mut UncomparedFields,
) -> io::Result<Option<DiffKind>> {
    match data_kind {
        DataKind::FILE => {
            if options.enabled(CompareField::Size, data_kind) {
                // fSIZ is a recorded size, not a measured one, so it settles the
                // question only when the caller asked about size.
                match entry.metadata().raw_file_size() {
                    Some(archive_size) if archive_size != fs_meta.len() as u128 => {
                        return Ok(Some(DiffKind::SizeDiffers));
                    }
                    Some(_) => {}
                    None => uncompared.record_entry(
                        CompareField::Size,
                        Uncomparable::NotRecorded,
                        path.as_str(),
                    ),
                }
            }
            if options.enabled(CompareField::Content, data_kind)
                && !streams_equal(fs::File::open(path)?, entry.reader(read_options)?)?
            {
                return Ok(Some(DiffKind::ContentsDiffer));
            }
            Ok(None)
        }
        DataKind::SYMBOLIC_LINK if options.enabled(CompareField::Symlink, data_kind) => {
            let link = fs::read_link(path)?;
            let EntryContent::SymbolicLink(stored) = entry.content(read_options)? else {
                unreachable!("data_kind() returned SymbolicLink");
            };
            Ok((link.as_path() != Path::new(stored.as_str())).then_some(DiffKind::SymlinkDiffers))
        }
        DataKind::HARD_LINK if options.enabled(CompareField::Hardlink, data_kind) => {
            let EntryContent::HardLink(stored) = entry.content(read_options)? else {
                unreachable!("data_kind() returned HardLink");
            };
            match is_same_file(path, stored.as_str()) {
                Ok(true) => Ok(None),
                Ok(false) => Ok(Some(DiffKind::NotLinked {
                    target: stored.to_string(),
                })),
                Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Some(DiffKind::Missing)),
                Err(e) => Err(e),
            }
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(compare: &[CompareFieldArg], full_compare: bool) -> CompareOptions {
        CompareOptions::new(compare.to_vec(), full_compare, Format::Plain).0
    }

    #[test]
    fn default_profile_compares_size_and_content_for_files_only() {
        let o = options(&[], false);
        for field in [CompareField::Size, CompareField::Content] {
            assert!(o.enabled(field, DataKind::FILE));
            assert!(!o.enabled(field, DataKind::DIRECTORY));
            assert!(!o.enabled(field, DataKind::SYMBOLIC_LINK));
            assert!(!o.enabled(field, DataKind::HARD_LINK));
        }
    }

    #[test]
    fn default_profile_compares_link_targets_for_matching_kind_only() {
        let o = options(&[], false);
        assert!(o.enabled(CompareField::Symlink, DataKind::SYMBOLIC_LINK));
        assert!(!o.enabled(CompareField::Symlink, DataKind::HARD_LINK));
        assert!(o.enabled(CompareField::Hardlink, DataKind::HARD_LINK));
        assert!(!o.enabled(CompareField::Hardlink, DataKind::SYMBOLIC_LINK));
    }

    #[cfg(unix)]
    #[test]
    fn default_profile_compares_directory_mode_without_full_compare() {
        let o = options(&[], false);
        assert!(o.enabled(CompareField::Mode, DataKind::FILE));
        assert!(o.enabled(CompareField::Mode, DataKind::DIRECTORY));
    }

    #[test]
    fn default_profile_compares_directory_mtime_only_with_full_compare() {
        assert!(!options(&[], false).enabled(CompareField::Mtime, DataKind::DIRECTORY));
        assert!(options(&[], true).enabled(CompareField::Mtime, DataKind::DIRECTORY));
        assert!(options(&[], false).enabled(CompareField::Mtime, DataKind::FILE));
    }

    #[cfg(unix)]
    #[test]
    fn default_profile_compares_directory_ownership_only_with_full_compare() {
        for field in [CompareField::Uid, CompareField::Gid] {
            assert!(!options(&[], false).enabled(field, DataKind::DIRECTORY));
            assert!(options(&[], true).enabled(field, DataKind::DIRECTORY));
            assert!(options(&[], false).enabled(field, DataKind::FILE));
        }
    }

    #[test]
    fn mtime_is_compared_on_every_platform() {
        assert!(CompareField::Mtime.is_supported());
        assert!(options(&[], false).enabled(CompareField::Mtime, DataKind::FILE));
    }

    #[test]
    fn explicit_field_suppresses_the_default_profile() {
        let o = options(&[CompareFieldArg::Size], false);
        assert!(o.enabled(CompareField::Size, DataKind::FILE));
        assert!(!o.enabled(CompareField::Content, DataKind::FILE));
        assert!(!o.enabled(CompareField::Symlink, DataKind::SYMBOLIC_LINK));
    }

    #[test]
    fn default_value_adds_the_default_profile_to_explicit_fields() {
        let o = options(&[CompareFieldArg::Default, CompareFieldArg::Mtime], false);
        assert!(o.enabled(CompareField::Content, DataKind::FILE));
        assert!(o.enabled(CompareField::Mtime, DataKind::DIRECTORY));
    }

    #[test]
    fn default_value_alone_is_the_same_as_omitting_the_option() {
        let explicit = options(&[CompareFieldArg::Default], false);
        let implicit = options(&[], false);
        for field in [
            CompareField::Size,
            CompareField::Content,
            CompareField::Mtime,
            CompareField::Mode,
            CompareField::Uid,
            CompareField::Gid,
            CompareField::Symlink,
            CompareField::Hardlink,
        ] {
            for kind in [
                DataKind::FILE,
                DataKind::DIRECTORY,
                DataKind::SYMBOLIC_LINK,
                DataKind::HARD_LINK,
            ] {
                assert_eq!(
                    explicit.enabled(field, kind),
                    implicit.enabled(field, kind),
                    "{field:?} on {kind:?}"
                );
            }
        }
    }

    #[test]
    fn repeated_fields_are_stored_once() {
        let o = options(&[CompareFieldArg::Size, CompareFieldArg::Size], false);
        assert_eq!(o.fields, CompareFields::SIZE);
    }

    #[test]
    fn omitting_the_option_compares_something() {
        assert!(!options(&[], false).compares_nothing());
    }

    #[cfg(not(unix))]
    #[test]
    fn unsupported_fields_are_dropped_and_leave_nothing_to_compare() {
        let o = options(&[CompareFieldArg::Uid], false);
        assert!(o.fields.is_empty());
        assert!(o.compares_nothing());
    }

    #[test]
    fn uncompared_fields_is_empty_until_something_is_recorded() {
        let mut uncompared = UncomparedFields::new(CompareFields::empty());
        assert!(uncompared.is_empty());
        uncompared.record_platform(CompareField::Uid);
        assert!(!uncompared.is_empty());
    }

    #[test]
    fn uncompared_fields_counts_entries_and_keeps_the_first_path() {
        let mut uncompared = UncomparedFields::new(CompareFields::SIZE);
        uncompared.record_entry(CompareField::Size, Uncomparable::NotRecorded, "a.txt");
        uncompared.record_entry(CompareField::Size, Uncomparable::NotRecorded, "b.txt");
        uncompared.record_platform(CompareField::Uid);

        assert_eq!(
            uncompared.to_string(),
            "cannot compare the requested field(s):\n  \
             size: not recorded in the archive (2 entries, e.g. a.txt)\n  \
             uid: unsupported on this platform"
        );
    }
}
