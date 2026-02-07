use crate::{
    cli::{FileArgs, PasswordArgs},
    command::{
        Command, ask_password,
        core::{SplitArchiveReader, collect_split_archives},
    },
    utils::{BsdGlobMatcher, io::streams_equal},
};

use clap::Parser;
use pna::prelude::MetadataTimeExt;
use pna::{DataKind, NormalEntry, ReadOptions};
use same_file::is_same_file;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::time::SystemTime;
use std::{
    fmt, fs,
    io::{self, prelude::*},
    path::Path,
};

#[derive(Parser, Clone, Debug)]
pub(crate) struct DiffCommand {
    #[command(flatten)]
    file: FileArgs,
    #[command(flatten)]
    password: PasswordArgs,
    #[arg(
        long,
        help = "Compare directory mtime and ownership (by default, only mode is compared for directories)"
    )]
    full_compare: bool,
}

impl Command for DiffCommand {
    #[inline]
    fn execute(self, _ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        diff_archive(self)
    }
}

#[hooq::hooq(anyhow)]
fn diff_archive(args: DiffCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let archives = collect_split_archives(&args.file.archive)?;
    let options = CompareOptions {
        full_compare: args.full_compare,
    };

    let mut globs = BsdGlobMatcher::new(args.file.files.iter().map(|s| s.as_str()));
    let filter_enabled = !globs.is_empty();

    let mut source = SplitArchiveReader::new(archives)?;
    source.for_each_entry(
        password.as_deref(),
        #[hooq::skip_all]
        |entry| {
            let entry = entry?;
            let path = entry.header().path();

            if filter_enabled && !globs.matches(path) {
                return Ok(());
            }

            compare_entry(entry, password.as_deref(), &options)
        },
    )?;

    globs.ensure_all_matched()?;

    Ok(())
}

/// Difference types detected during archive-filesystem comparison.
/// Message format follows tar --diff for compatibility.
#[derive(Clone, Debug, PartialEq, Eq)]
enum DiffKind {
    /// File/directory does not exist on filesystem
    Missing,
    /// File size differs
    SizeDiffers,
    /// File contents differ (same size)
    ContentsDiffer,
    /// Permission mode differs
    ModeDiffers,
    /// Modification time differs
    MtimeDiffers,
    /// User ID differs
    UidDiffers,
    /// Group ID differs
    GidDiffers,
    /// File type mismatch (e.g., file vs directory)
    TypeMismatch,
    /// Symbolic link target differs
    SymlinkDiffers,
    /// Hardlink relationship broken
    NotLinked(String),
}

impl DiffKind {
    /// Returns a displayable message for this difference.
    fn display<'a>(&'a self, path: &'a str) -> DiffMessage<'a> {
        DiffMessage { kind: self, path }
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
            DiffKind::ModeDiffers => write!(f, "{}: Mode differs", self.path),
            DiffKind::MtimeDiffers => write!(f, "{}: Mod time differs", self.path),
            DiffKind::UidDiffers => write!(f, "{}: Uid differs", self.path),
            DiffKind::GidDiffers => write!(f, "{}: Gid differs", self.path),
            DiffKind::TypeMismatch => write!(f, "{}: File type mismatch", self.path),
            DiffKind::SymlinkDiffers => write!(f, "{}: Symlink differs", self.path),
            DiffKind::NotLinked(target) => write!(f, "{}: Not linked to {target}", self.path),
        }
    }
}

/// Options controlling what aspects to compare.
#[derive(Clone, Debug, Default)]
struct CompareOptions {
    /// Compare directory mtime and ownership (not just mode)
    full_compare: bool,
}

/// Compare two SystemTime values with 1-second tolerance for filesystem precision.
fn times_equal(a: SystemTime, b: SystemTime) -> bool {
    match a.duration_since(b) {
        Ok(d) => d.as_secs() == 0,
        Err(e) => e.duration().as_secs() == 0,
    }
}

/// Compare file metadata and return list of differences.
#[cfg(unix)]
fn compare_file_metadata<T: AsRef<[u8]>>(
    entry: &NormalEntry<T>,
    fs_meta: &fs::Metadata,
    _options: &CompareOptions,
) -> Vec<DiffKind> {
    let mut diffs = Vec::new();

    // Compare mode
    if let Some(perm) = entry.metadata().permission() {
        let archive_mode = perm.permissions() & 0o7777;
        let fs_mode = (fs_meta.permissions().mode() & 0o7777) as u16;
        if archive_mode != fs_mode {
            diffs.push(DiffKind::ModeDiffers);
        }
    }

    // Compare mtime
    if let Some(archive_mtime) = entry.metadata().modified_time()
        && let Ok(fs_mtime) = fs_meta.modified()
        && !times_equal(archive_mtime, fs_mtime)
    {
        diffs.push(DiffKind::MtimeDiffers);
    }

    // Compare uid/gid
    if let Some(perm) = entry.metadata().permission() {
        if perm.uid() != fs_meta.uid() as u64 {
            diffs.push(DiffKind::UidDiffers);
        }
        if perm.gid() != fs_meta.gid() as u64 {
            diffs.push(DiffKind::GidDiffers);
        }
    }

    diffs
}

#[cfg(not(unix))]
fn compare_file_metadata<T: AsRef<[u8]>>(
    _entry: &NormalEntry<T>,
    _fs_meta: &fs::Metadata,
    _options: &CompareOptions,
) -> Vec<DiffKind> {
    Vec::new()
}

/// Compare directory metadata and return list of differences.
/// By default only compares mode. With full_compare, also checks mtime and ownership.
#[cfg(unix)]
fn compare_directory_metadata<T: AsRef<[u8]>>(
    entry: &NormalEntry<T>,
    fs_meta: &fs::Metadata,
    options: &CompareOptions,
) -> Vec<DiffKind> {
    let mut diffs = Vec::new();

    // Always compare mode for directories
    if let Some(perm) = entry.metadata().permission() {
        let archive_mode = perm.permissions() & 0o7777;
        let fs_mode = (fs_meta.permissions().mode() & 0o7777) as u16;
        if archive_mode != fs_mode {
            diffs.push(DiffKind::ModeDiffers);
        }
    }

    // Only compare mtime and ownership with --full-compare
    if options.full_compare {
        if let Some(archive_mtime) = entry.metadata().modified_time()
            && let Ok(fs_mtime) = fs_meta.modified()
            && !times_equal(archive_mtime, fs_mtime)
        {
            diffs.push(DiffKind::MtimeDiffers);
        }

        if let Some(perm) = entry.metadata().permission() {
            if perm.uid() != fs_meta.uid() as u64 {
                diffs.push(DiffKind::UidDiffers);
            }
            if perm.gid() != fs_meta.gid() as u64 {
                diffs.push(DiffKind::GidDiffers);
            }
        }
    }

    diffs
}

#[cfg(not(unix))]
fn compare_directory_metadata<T: AsRef<[u8]>>(
    _entry: &NormalEntry<T>,
    _fs_meta: &fs::Metadata,
    _options: &CompareOptions,
) -> Vec<DiffKind> {
    Vec::new()
}

fn compare_entry<T: AsRef<[u8]>>(
    entry: NormalEntry<T>,
    password: Option<&[u8]>,
    options: &CompareOptions,
) -> io::Result<()> {
    let data_kind = entry.header().data_kind();
    let path = entry.header().path();
    let path_str = path.as_str();
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            println!("{}", DiffKind::Missing.display(path_str));
            return Ok(());
        }
        Err(e) => return Err(e),
    };
    match data_kind {
        DataKind::File if meta.is_file() => {
            // Compare metadata first
            let meta_diffs = compare_file_metadata(&entry, &meta, options);
            for diff in meta_diffs {
                println!("{}", diff.display(path_str));
            }

            // Compare size first, then content
            let fs_size = meta.len();
            let archive_size = entry.metadata().raw_file_size();
            if archive_size.is_some_and(|s| s != fs_size as u128) {
                println!("{}", DiffKind::SizeDiffers.display(path_str));
            } else {
                let fs_file = fs::File::open(path)?;
                let archive_reader = entry.reader(ReadOptions::with_password(password))?;
                if !streams_equal(fs_file, archive_reader)? {
                    println!("{}", DiffKind::ContentsDiffer.display(path_str));
                }
            }
        }
        DataKind::Directory if meta.is_dir() => {
            let diffs = compare_directory_metadata(&entry, &meta, options);
            for diff in diffs {
                println!("{}", diff.display(path_str));
            }
        }
        DataKind::SymbolicLink if meta.is_symlink() => {
            let link = fs::read_link(path)?;
            let mut reader = entry.reader(ReadOptions::with_password(password))?;
            let mut link_str = String::new();
            reader.read_to_string(&mut link_str)?;
            if link.as_path() != Path::new(&link_str) {
                println!("{}", DiffKind::SymlinkDiffers.display(path_str));
            }
        }
        DataKind::File | DataKind::Directory | DataKind::SymbolicLink => {
            println!("{}", DiffKind::TypeMismatch.display(path_str));
        }
        DataKind::HardLink if meta.is_file() => {
            let mut reader = entry.reader(ReadOptions::with_password(password))?;
            let mut target = String::new();
            reader.read_to_string(&mut target)?;

            match is_same_file(path, &target) {
                Ok(true) => (),
                Ok(false) => {
                    println!("{}", DiffKind::NotLinked(target).display(path_str));
                }
                Err(e) if e.kind() == io::ErrorKind::NotFound => {
                    println!("{}", DiffKind::Missing.display(path_str));
                }
                Err(e) => return Err(e),
            }
        }
        DataKind::HardLink => {
            println!("{}", DiffKind::TypeMismatch.display(path_str));
        }
    }
    Ok(())
}
