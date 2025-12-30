#[cfg(feature = "memmap")]
use crate::command::core::run_read_entries_mem;
use crate::{
    chunk,
    cli::{ColorChoice, FileArgsCompat, PasswordArgs},
    command::{
        Command, ask_password,
        core::{PathFilter, collect_split_archives, read_paths, run_read_entries},
    },
    ext::*,
    utils::{GlobPatterns, VCS_FILES},
};
use base64::Engine;
use chrono::{
    DateTime, Local,
    format::{DelayedFormat, StrftimeItems},
};
use clap::{
    ArgGroup, Parser, ValueEnum, ValueHint,
    builder::styling::{AnsiColor, Color as Colour, Style},
};
use pna::{
    Compression, DataKind, Encryption, ExtendedAttribute, NormalEntry, RawChunk, ReadEntry,
    ReadOptions, SolidHeader, prelude::*,
};
use rayon::prelude::*;
use serde::Serialize;
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    fmt::{self, Display, Formatter},
    io::{self, prelude::*},
    path::PathBuf,
    time::{Duration, SystemTime},
};
use tabled::{
    builder::Builder as TableBuilder,
    settings::{
        Alignment, Color, Modify, Padding, PaddingColor, Style as TableStyle,
        object::{Rows, Segment},
        themes::Colorization,
    },
};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[clap(disable_help_flag = true)]
#[command(
    group(ArgGroup::new("null-requires").arg("null").requires("exclude_from")),
)]
pub(crate) struct ListCommand {
    #[arg(short, long, help = "Display extended file metadata as a table")]
    pub(crate) long: bool,
    #[arg(short, long, help = "Add a header row to each column")]
    pub(crate) header: bool,
    #[arg(long, help = "Display solid mode archive entries")]
    pub(crate) solid: bool,
    #[arg(short = '@', help = "Display extended file attributes in a table")]
    pub(crate) show_xattr: bool,
    #[arg(
        short = 'e',
        requires = "unstable",
        help = "Display acl in a table (unstable)"
    )]
    show_acl: bool,
    #[arg(
        long = "private",
        requires = "unstable",
        help = "Display private chunks in a table (unstable)"
    )]
    pub(crate) show_private: bool,
    #[arg(
        long,
        help = "Display user id and group id instead of user name and group name"
    )]
    pub(crate) numeric_owner: bool,
    #[arg(
        short = 'T',
        help = "When used with the -l option, display complete time information for the entry, including month, day, hour, minute, second, and year"
    )]
    pub(crate) long_time: bool,
    #[arg(long, help = "Display format [unstable: jsonl, bsdtar, csv, tsv]")]
    format: Option<Format>,
    #[arg(
        long,
        help = "Which timestamp field to list (modified, accessed, created)"
    )]
    time: Option<TimeField>,
    #[arg(
        short = 'q',
        help = "Force printing of non-graphic characters in file names as the character '?'"
    )]
    hide_control_chars: bool,
    #[arg(long, help = "Display type indicator by entry kinds")]
    classify: bool,
    #[arg(
        long,
        help = "Process only files or directories that match the specified pattern. Note that exclusions specified with --exclude take precedence over inclusions"
    )]
    include: Option<Vec<String>>,
    #[arg(
        long,
        requires = "unstable",
        help = "Exclude path glob (unstable)",
        value_hint = ValueHint::AnyPath
    )]
    exclude: Option<Vec<String>>,
    #[arg(
        long,
        requires = "unstable",
        help = "Read exclude files from given path (unstable)",
        value_hint = ValueHint::FilePath
    )]
    exclude_from: Option<PathBuf>,
    #[arg(
        long,
        requires = "unstable",
        help = "Exclude files or directories internally used by version control systems (`Arch`, `Bazaar`, `CVS`, `Darcs`, `Mercurial`, `RCS`, `SCCS`, `SVN`, `git`) (unstable)"
    )]
    exclude_vcs: bool,
    #[arg(
        long,
        help = "Filenames or patterns are separated by null characters, not by newlines"
    )]
    null: bool,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) file: FileArgsCompat,
    #[arg(long, action = clap::ArgAction::Help, help = "Print help")]
    help: (),
}

impl Command for ListCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalArgs) -> anyhow::Result<()> {
        if let Some(format) = self.format {
            if format.is_unstable() && !ctx.unstable {
                anyhow::bail!(
                    "The '--format {}' option is unstable and requires --unstable flag",
                    format
                );
            }
        }
        list_archive(self, ctx.color())
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, ValueEnum)]
#[value(rename_all = "lower")]
pub(crate) enum Format {
    Line,
    Table,
    JsonL,
    Tree,
    BsdTar,
    Csv,
    Tsv,
}

impl Format {
    /// Returns true if this format is unstable and requires --unstable flag
    #[inline]
    const fn is_unstable(self) -> bool {
        matches!(self, Self::JsonL | Self::BsdTar | Self::Csv | Self::Tsv)
    }
}

impl fmt::Display for Format {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.to_possible_value().unwrap().get_name())
    }
}

#[derive(Debug)]
enum EntryType {
    File(String),
    Directory(String),
    SymbolicLink(String, String),
    HardLink(String, String),
    BlockDevice(String),
    CharDevice(String),
    Fifo(String),
}

impl EntryType {
    #[inline]
    fn name(&self) -> &str {
        match self {
            EntryType::File(name)
            | EntryType::Directory(name)
            | EntryType::SymbolicLink(name, _)
            | EntryType::HardLink(name, _)
            | EntryType::BlockDevice(name)
            | EntryType::CharDevice(name)
            | EntryType::Fifo(name) => name,
        }
    }

    #[inline]
    fn bsd_long_style_display(&self) -> EntryTypeBsdLongStyleDisplay<'_> {
        EntryTypeBsdLongStyleDisplay(self)
    }
}

struct EntryTypeBsdLongStyleDisplay<'a>(&'a EntryType);

impl Display for EntryTypeBsdLongStyleDisplay<'_> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.0 {
            EntryType::File(name) => Display::fmt(&name, f),
            EntryType::Directory(name) => write!(f, "{name}/"),
            EntryType::SymbolicLink(name, link_to) => {
                write!(f, "{name} -> {link_to}")
            }
            EntryType::HardLink(name, link_to) => {
                write!(f, "{name} link to {link_to}")
            }
            EntryType::BlockDevice(name) => write!(f, "{name} (block device)"),
            EntryType::CharDevice(name) => write!(f, "{name} (char device)"),
            EntryType::Fifo(name) => write!(f, "{name} (fifo)"),
        }
    }
}

struct TableRow {
    encryption: String,
    compression: String,
    permission: Option<pna::Permission>,
    raw_size: Option<u128>,
    compressed_size: usize,
    created: Option<SystemTime>,
    modified: Option<SystemTime>,
    accessed: Option<SystemTime>,
    entry_type: EntryType,
    xattrs: Vec<ExtendedAttribute>,
    acl: HashMap<chunk::AcePlatform, Vec<chunk::Ace>>,
    privates: Vec<RawChunk>,
}

impl TableRow {
    #[inline]
    fn permission_mode(&self) -> u16 {
        self.permission.as_ref().map_or(0, |it| it.permissions())
    }
}

impl<T> TryFrom<(&NormalEntry<T>, Option<&[u8]>, Option<&SolidHeader>)> for TableRow
where
    T: AsRef<[u8]> + Clone,
    RawChunk<T>: Chunk,
    RawChunk: From<RawChunk<T>>,
{
    type Error = io::Error;
    #[inline]
    fn try_from(
        (entry, password, solid): (&NormalEntry<T>, Option<&[u8]>, Option<&SolidHeader>),
    ) -> Result<Self, Self::Error> {
        let metadata = entry.metadata();
        let acl = entry.acl()?;
        Ok(Self {
            encryption: match solid.map_or_else(
                || (entry.encryption(), entry.cipher_mode()),
                |s| (s.encryption(), s.cipher_mode()),
            ) {
                (Encryption::No, _) => "-".into(),
                (encryption, cipher_mode) => {
                    format!("{encryption:?}({cipher_mode:?})").to_ascii_lowercase()
                }
            },
            compression: match (
                solid.map_or(entry.compression(), |s| s.compression()),
                solid,
            ) {
                (Compression::No, None) => "-".into(),
                (Compression::No, Some(_)) => "-(solid)".into(),
                (method, None) => format!("{method:?}").to_ascii_lowercase(),
                (method, Some(_)) => format!("{method:?}(solid)").to_ascii_lowercase(),
            },
            permission: metadata.permission().cloned(),
            raw_size: metadata.raw_file_size(),
            compressed_size: metadata.compressed_size(),
            created: metadata.created_time(),
            modified: metadata.modified_time(),
            accessed: metadata.accessed_time(),
            entry_type: match entry.data_kind() {
                DataKind::SymbolicLink => EntryType::SymbolicLink(
                    entry.name().to_string(),
                    entry
                        .reader(ReadOptions::with_password(password))
                        .and_then(io::read_to_string)
                        .unwrap_or_else(|_| "-".into()),
                ),
                DataKind::HardLink => EntryType::HardLink(
                    entry.name().to_string(),
                    entry
                        .reader(ReadOptions::with_password(password))
                        .and_then(io::read_to_string)
                        .unwrap_or_else(|_| "-".into()),
                ),
                DataKind::Directory => EntryType::Directory(entry.name().to_string()),
                DataKind::File => EntryType::File(entry.name().to_string()),
                DataKind::BlockDevice => EntryType::BlockDevice(entry.name().to_string()),
                DataKind::CharDevice => EntryType::CharDevice(entry.name().to_string()),
                DataKind::Fifo => EntryType::Fifo(entry.name().to_string()),
            },
            xattrs: entry.xattrs().to_vec(),
            acl,
            privates: entry
                .extra_chunks()
                .iter()
                .filter(|it| it.ty() != chunk::faCe && it.ty() != chunk::faCl)
                .map(|it| (*it).clone().into())
                .collect::<Vec<_>>(),
        })
    }
}

fn list_archive(args: ListCommand, color: ColorChoice) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let options = ListOptions {
        long: args.long,
        header: args.header,
        solid: args.solid,
        show_xattr: args.show_xattr,
        show_acl: args.show_acl,
        show_private: args.show_private,
        time_format: if args.long_time {
            TimeFormat::Long
        } else {
            TimeFormat::Auto(SystemTime::now())
        },
        time_field: args.time.unwrap_or_default(),
        numeric_owner: args.numeric_owner,
        hide_control_chars: args.hide_control_chars,
        classify: args.classify,
        format: args.format,
        out_to_stderr: false,
        color,
    };
    let archive = args.file.archive();
    let files = args.file.files();
    let files_globs = GlobPatterns::new(files.iter().map(|it| it.as_str()))?;

    let mut exclude = args.exclude.unwrap_or_default();
    if let Some(p) = args.exclude_from {
        exclude.extend(read_paths(p, args.null)?);
    }
    let vcs_patterns = args
        .exclude_vcs
        .then(|| VCS_FILES.iter().copied())
        .into_iter()
        .flatten();
    let filter = PathFilter::new(
        args.include.iter().flatten(),
        exclude.iter().map(|s| s.as_str()).chain(vcs_patterns),
    );

    let archives = collect_split_archives(&archive)?;

    #[cfg(not(feature = "memmap"))]
    {
        run_list_archive(
            archives
                .into_iter()
                .map(|it| io::BufReader::with_capacity(64 * 1024, it)),
            password.as_deref(),
            files_globs,
            filter,
            options,
        )
    }
    #[cfg(feature = "memmap")]
    {
        run_list_archive_mem(archives, password.as_deref(), files_globs, filter, options)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum TimeFormat {
    Auto(SystemTime),
    Long,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, ValueEnum)]
#[value(rename_all = "lower")]
pub(crate) enum TimeField {
    Created,
    #[default]
    Modified,
    Accessed,
}

impl TimeField {
    #[inline]
    const fn as_str(&self) -> &'static str {
        match self {
            TimeField::Created => "Created",
            TimeField::Modified => "Modified",
            TimeField::Accessed => "Accessed",
        }
    }
}

pub(crate) struct ListOptions {
    pub(crate) long: bool,
    pub(crate) header: bool,
    pub(crate) solid: bool,
    pub(crate) show_xattr: bool,
    pub(crate) show_acl: bool,
    pub(crate) show_private: bool,
    pub(crate) time_format: TimeFormat,
    pub(crate) time_field: TimeField,
    pub(crate) numeric_owner: bool,
    pub(crate) hide_control_chars: bool,
    pub(crate) classify: bool,
    pub(crate) format: Option<Format>,
    pub(crate) out_to_stderr: bool,
    pub(crate) color: ColorChoice,
}

pub(crate) fn run_list_archive<'a>(
    archive_provider: impl IntoIterator<Item = impl Read>,
    password: Option<&[u8]>,
    files_globs: GlobPatterns,
    filter: PathFilter<'a>,
    args: ListOptions,
) -> anyhow::Result<()> {
    let mut entries = Vec::new();

    run_read_entries(archive_provider, |entry| {
        match entry? {
            ReadEntry::Solid(solid) if args.solid => {
                for entry in solid.entries(password)? {
                    entries.push((&entry?, password, Some(solid.header())).try_into()?)
                }
            }
            ReadEntry::Solid(_) => {
                log::warn!(
                    "This archive contain solid mode entry. if you need to show it use --solid option."
                );
            }
            ReadEntry::Normal(item) => entries.push((&item, password, None).try_into()?),
        }
        Ok(())
    })?;
    print_entries(entries, files_globs, filter, args)
}

#[cfg(feature = "memmap")]
pub(crate) fn run_list_archive_mem<'a>(
    archives: Vec<std::fs::File>,
    password: Option<&[u8]>,
    files_globs: GlobPatterns,
    filter: PathFilter<'a>,
    args: ListOptions,
) -> anyhow::Result<()> {
    let mut entries = Vec::new();
    let mmaps = archives
        .into_iter()
        .map(crate::utils::mmap::Mmap::try_from)
        .collect::<io::Result<Vec<_>>>()?;
    let archives = mmaps.iter().map(|m| m.as_ref());

    run_read_entries_mem(archives, |entry| {
        match entry? {
            ReadEntry::Solid(solid) if args.solid => {
                for entry in solid.entries(password)? {
                    entries.push((&entry?, password, Some(solid.header())).try_into()?);
                }
            }
            ReadEntry::Solid(_) => {
                log::warn!(
                    "This archive contain solid mode entry. if you need to show it use --solid option."
                );
            }
            ReadEntry::Normal(item) => entries.push((&item, password, None).try_into()?),
        }
        Ok(())
    })?;
    print_entries(entries, files_globs, filter, args)
}

fn print_entries<'a>(
    entries: Vec<TableRow>,
    mut globs: GlobPatterns,
    filter: PathFilter<'a>,
    options: ListOptions,
) -> anyhow::Result<()> {
    let entries = entries
        .into_iter()
        .filter(|r| {
            (globs.is_empty() || globs.matches_any(r.entry_type.name()))
                && !filter.excluded(r.entry_type.name())
        })
        .collect::<Vec<_>>();
    globs.ensure_all_matched()?;
    if options.out_to_stderr {
        let out = anstream::AutoStream::new(io::stderr().lock(), options.color.into());
        print_formatted_entries(entries, &options, out)
    } else {
        let out = anstream::AutoStream::new(io::stdout().lock(), options.color.into());
        print_formatted_entries(entries, &options, out)
    }
}

fn print_formatted_entries(
    entries: Vec<TableRow>,
    options: &ListOptions,
    out: impl Write,
) -> anyhow::Result<()> {
    match options.format {
        Some(Format::Line) => simple_list_entries_to(entries, options, out)?,
        Some(Format::JsonL) => json_line_entries_to(entries, out)?,
        Some(Format::Table) => detail_list_entries_to(entries, options, out)?,
        Some(Format::Tree) => tree_entries_to(entries, options, out)?,
        Some(Format::BsdTar) => bsd_tar_list_entries_to(entries, options, out)?,
        Some(Format::Csv) => csv_entries_to(entries, options, out)?,
        Some(Format::Tsv) => tsv_entries_to(entries, options, out)?,
        None if options.long => detail_list_entries_to(entries, options, out)?,
        None => simple_list_entries_to(entries, options, out)?,
    };
    Ok(())
}

fn bsd_tar_list_entries_to(
    entries: Vec<TableRow>,
    options: &ListOptions,
    mut out: impl Write,
) -> io::Result<()> {
    let now = SystemTime::now();
    let mut uname_width = 6;
    let mut gname_width = 6;
    for row in entries {
        let nlink = 0; // BSD tar show always 0
        let permission = row.permission_mode();
        let has_xattr = !row.xattrs.is_empty();
        let has_acl = !row.acl.is_empty();
        let perm = permission_string(&row.entry_type, permission, has_xattr, has_acl);
        let size = row.raw_size.unwrap_or(0);
        let mtime = bsd_tar_time(now, row.modified.unwrap_or(now));
        let (uname, gname) = match &row.permission {
            Some(p) => (
                if options.numeric_owner || p.uname().is_empty() {
                    Cow::Owned(p.uid().to_string())
                } else {
                    Cow::Borrowed(p.uname())
                },
                if options.numeric_owner || p.gname().is_empty() {
                    Cow::Owned(p.gid().to_string())
                } else {
                    Cow::Borrowed(p.gname())
                },
            ),
            None => (Cow::default(), Cow::default()),
        };
        let name = row.entry_type.bsd_long_style_display();
        uname_width = uname_width.max(uname.len());
        gname_width = gname_width.max(gname.len());

        // permission nlink uname gname size mtime name link
        // ex: -rw-r--r--  0 1000   1000        0 Jan  1  1980 f
        writeln!(
            out,
            "{perm}  {nlink} {uname:<uname_width$} {gname:<gname_width$} {size:8} {mtime} {name}"
        )?;
    }
    Ok(())
}

fn bsd_tar_time(now: SystemTime, time: SystemTime) -> DelayedFormat<StrftimeItems<'static>> {
    let datetime = DateTime::<Local>::from(time);
    if within_six_months(now, time) {
        datetime.format("%b %e %H:%M")
    } else {
        datetime.format("%b %e  %Y")
    }
}

struct SimpleListDisplay<'a> {
    entries: &'a [TableRow],
    options: &'a ListOptions,
}

impl<'a> Display for SimpleListDisplay<'a> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use core::fmt::Write;
        self.entries.iter().try_for_each(|path| {
            let name = path.entry_type.name();
            if self.options.hide_control_chars {
                Display::fmt(&hide_control_chars(name), f)
            } else {
                Display::fmt(name, f)
            }?;
            match &path.entry_type {
                EntryType::Directory(_) if self.options.classify => f.write_char('/')?,
                EntryType::SymbolicLink(_, _) if self.options.classify => f.write_char('@')?,
                _ => (),
            };
            f.write_char('\n')
        })
    }
}

fn simple_list_entries_to(
    entries: Vec<TableRow>,
    options: &ListOptions,
    mut out: impl Write,
) -> io::Result<()> {
    let display = SimpleListDisplay {
        entries: &entries,
        options,
    };
    write!(out, "{display}")
}

fn detail_list_entries_to(
    entries: impl IntoIterator<Item = TableRow>,
    options: &ListOptions,
    mut out: impl Write,
) -> io::Result<()> {
    let underline = Color::new("\x1B[4m", "\x1B[0m");
    let reset = Color::new("\x1B[8m", "\x1B[0m");
    let header = [
        "Encryption",
        "Compression",
        "Permissions",
        "Raw Size",
        "Compressed Size",
        "User",
        "Group",
        options.time_field.as_str(),
        "Name",
    ];
    let mut acl_rows = Vec::new();
    let mut xattr_rows = Vec::new();
    let mut builder = TableBuilder::new();
    builder.set_empty(String::new());
    if options.header {
        builder.push_record(header);
    }
    for content in entries {
        let has_acl = !content.acl.is_empty();
        let has_xattr = !content.xattrs.is_empty();
        let permission_mode = content.permission_mode();
        let user = content.permission.as_ref().map_or_else(
            || "-".into(),
            |it| it.owner_display(options.numeric_owner).to_string(),
        );
        let group = content.permission.as_ref().map_or_else(
            || "-".into(),
            |it| it.group_display(options.numeric_owner).to_string(),
        );
        builder.push_record([
            content.encryption,
            content.compression,
            paint_permission(&content.entry_type, permission_mode, has_xattr, has_acl),
            content
                .raw_size
                .map_or_else(|| "-".into(), |size| size.to_string()),
            content.compressed_size.to_string(),
            user,
            group,
            match options.time_field {
                TimeField::Created => content.created,
                TimeField::Modified => content.modified,
                TimeField::Accessed => content.accessed,
            }
            .map_or_else(|| "-".into(), |d| datetime(options.time_format, d)),
            {
                let name = match content.entry_type {
                    EntryType::Directory(path) if options.classify => format!("{path}/"),
                    EntryType::SymbolicLink(name, link_to) if options.classify => {
                        format!("{name}@ -> {link_to}")
                    }
                    EntryType::File(path)
                    | EntryType::Directory(path)
                    | EntryType::BlockDevice(path)
                    | EntryType::CharDevice(path)
                    | EntryType::Fifo(path) => path,
                    EntryType::SymbolicLink(path, link_to) | EntryType::HardLink(path, link_to) => {
                        format!("{path} -> {link_to}")
                    }
                };
                if options.hide_control_chars {
                    hide_control_chars(&name).to_string()
                } else {
                    name
                }
            },
        ]);
        if options.show_acl {
            let acl = content.acl.into_iter().flat_map(|(platform, ace)| {
                ace.into_iter().map(move |it| chunk::AceWithPlatform {
                    platform: Some(platform.clone()),
                    ace: it,
                })
            });
            for a in acl {
                builder.push_record([String::new(), String::new(), a.to_string()]);
                acl_rows.push(builder.count_records());
            }
        }
        if options.show_xattr {
            for x in &content.xattrs {
                builder.push_record([
                    String::new(),
                    String::new(),
                    x.name().into(),
                    x.value().len().to_string(),
                ]);
                xattr_rows.push(builder.count_records());
            }
        }
        if options.show_private {
            for c in &content.privates {
                builder.push_record([
                    String::new(),
                    String::new(),
                    format!("chunk:{}", c.ty()),
                    c.data().len().to_string(),
                ]);
            }
        }
    }
    let mut table = builder.build();
    table
        .with(TableStyle::empty())
        .with(Colorization::columns([
            Color::FG_MAGENTA,
            Color::FG_BLUE,
            Color::empty(),
            Color::FG_GREEN,
            Color::FG_GREEN,
            Color::FG_CYAN,
            Color::FG_CYAN,
            Color::FG_CYAN,
            Color::FG_CYAN,
            Color::empty(),
        ]))
        .with(Modify::new(Segment::new(.., 3..=4)).with(Alignment::right()));
    if options.header {
        table.with(Colorization::exact([underline], Rows::first()));
    }
    table.with(Padding::new(0, 1, 0, 0)).with(PaddingColor::new(
        Color::empty(),
        reset,
        Color::empty(),
        Color::empty(),
    ));
    writeln!(out, "{table}")
}

const DURATION_SIX_MONTH: Duration = Duration::from_secs(60 * 60 * 24 * 30 * 6);

fn within_six_months(now: SystemTime, x: SystemTime) -> bool {
    let six_months_ago = now - DURATION_SIX_MONTH;
    six_months_ago <= x
}

fn datetime(format: TimeFormat, time: SystemTime) -> String {
    let datetime = DateTime::<Local>::from(time);
    match format {
        TimeFormat::Auto(now) => {
            if within_six_months(now, time) {
                datetime.format("%b %e %H:%M")
            } else {
                datetime.format("%b %e  %Y")
            }
        }
        TimeFormat::Long => datetime.format("%b %e %H:%M:%S %Y"),
    }
    .to_string()
}

#[inline]
fn hide_control_chars<'a>(s: &'a str) -> impl Display + 'a {
    use core::fmt::Write;
    struct HideControl<'s>(&'s str);

    impl Display for HideControl<'_> {
        #[inline]
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            self.0.chars().try_for_each(|c| {
                if c.is_control() {
                    f.write_char('?')
                } else {
                    f.write_char(c)
                }
            })
        }
    }
    HideControl(s)
}

#[derive(Clone, Eq, PartialEq, Debug)]
struct StyledDisplay<'s, T> {
    style: &'s Style,
    v: T,
}

impl<T: Display> Display for StyledDisplay<'_, T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}{:#}", self.style, self.v, self.style)
    }
}

trait StyleExt<T> {
    fn paint(&self, v: T) -> StyledDisplay<'_, T>;
}

impl<T: Display> StyleExt<T> for Style {
    #[inline]
    fn paint(&self, v: T) -> StyledDisplay<'_, T> {
        StyledDisplay { style: self, v }
    }
}

const STYLE_READ: Style = Style::new().fg_color(Some(Colour::Ansi(AnsiColor::Yellow)));
const STYLE_WRITE: Style = Style::new().fg_color(Some(Colour::Ansi(AnsiColor::Red)));
const STYLE_EXEC: Style = Style::new().fg_color(Some(Colour::Ansi(AnsiColor::Blue)));
const STYLE_DIR: Style = Style::new().fg_color(Some(Colour::Ansi(AnsiColor::Magenta)));
const STYLE_LINK: Style = Style::new().fg_color(Some(Colour::Ansi(AnsiColor::Cyan)));
const STYLE_HYPHEN: Style = Style::new();

fn kind_paint(kind: &EntryType) -> impl Display + 'static {
    match kind {
        EntryType::File(_) | EntryType::HardLink(_, _) => STYLE_HYPHEN.paint('.'),
        EntryType::Directory(_) => STYLE_DIR.paint('d'),
        EntryType::SymbolicLink(_, _) => STYLE_LINK.paint('l'),
        EntryType::BlockDevice(_) => STYLE_HYPHEN.paint('b'),
        EntryType::CharDevice(_) => STYLE_HYPHEN.paint('c'),
        EntryType::Fifo(_) => STYLE_HYPHEN.paint('p'),
    }
}

fn paint_permission(kind: &EntryType, permission: u16, has_xattr: bool, has_acl: bool) -> String {
    let paint = |style: &'static Style, c: char, bit: u16| {
        if permission & bit != 0 {
            style.paint(c)
        } else {
            STYLE_HYPHEN.paint('-')
        }
    };

    format!(
        "{}{}{}{}{}{}{}{}{}{}{}",
        kind_paint(kind),
        paint(&STYLE_READ, 'r', 0b100000000),  // owner_read
        paint(&STYLE_WRITE, 'w', 0b010000000), // owner_write
        paint(&STYLE_EXEC, 'x', 0b001000000),  // owner_exec
        paint(&STYLE_READ, 'r', 0b000100000),  // group_read
        paint(&STYLE_WRITE, 'w', 0b000010000), // group_write
        paint(&STYLE_EXEC, 'x', 0b000001000),  // group_exec
        paint(&STYLE_READ, 'r', 0b000000100),  // other_read
        paint(&STYLE_WRITE, 'w', 0b000000010), // other_write
        paint(&STYLE_EXEC, 'x', 0b000000001),  // other_exec
        STYLE_HYPHEN.paint(if has_xattr {
            '@'
        } else if has_acl {
            '+'
        } else {
            ' '
        }),
    )
}

const fn kind_char(kind: &EntryType) -> char {
    match kind {
        EntryType::File(_) | EntryType::HardLink(_, _) => '-',
        EntryType::Directory(_) => 'd',
        EntryType::SymbolicLink(_, _) => 'l',
        EntryType::BlockDevice(_) => 'b',
        EntryType::CharDevice(_) => 'c',
        EntryType::Fifo(_) => 'p',
    }
}

fn permission_string(kind: &EntryType, permission: u16, has_xattr: bool, has_acl: bool) -> String {
    #[inline(always)]
    const fn paint(permission: u16, c: char, bit: u16) -> char {
        if permission & bit != 0 { c } else { '-' }
    }

    format!(
        "{}{}{}{}{}{}{}{}{}{}{}",
        kind_char(kind),
        paint(permission, 'r', 0b100000000), // owner_read
        paint(permission, 'w', 0b010000000), // owner_write
        paint(permission, 'x', 0b001000000), // owner_exec
        paint(permission, 'r', 0b000100000), // group_read
        paint(permission, 'w', 0b000010000), // group_write
        paint(permission, 'x', 0b000001000), // group_exec
        paint(permission, 'r', 0b000000100), // other_read
        paint(permission, 'w', 0b000000010), // other_write
        paint(permission, 'x', 0b000000001), // other_exec
        if has_xattr {
            '@'
        } else if has_acl {
            '+'
        } else {
            ' '
        },
    )
}

#[derive(Serialize, Debug)]
struct FileInfo<'a> {
    filename: &'a str,
    permissions: String,
    owner: String,
    group: String,
    raw_size: u128,
    size: usize,
    encryption: &'a str,
    compression: &'a str,
    created: String,
    modified: String,
    accessed: String,
    acl: Vec<AclEntry>,
    xattr: Vec<XAttr<'a>>,
}

#[derive(Serialize, Debug)]
struct AclEntry {
    platform: String,
    entries: Vec<String>,
}

#[derive(Serialize, Debug)]
struct XAttr<'a> {
    key: &'a str,
    value: String,
}

fn json_line_entries_to(entries: Vec<TableRow>, mut out: impl Write) -> anyhow::Result<()> {
    let entries = entries
        .par_iter()
        .map(|it| {
            let permission_mode = it.permission_mode();
            let owner = it
                .permission
                .as_ref()
                .map_or_else(String::new, |it| it.uname().to_string());
            let group = it
                .permission
                .as_ref()
                .map_or_else(String::new, |it| it.gname().to_string());
            FileInfo {
                filename: it.entry_type.name(),
                permissions: permission_string(
                    &it.entry_type,
                    permission_mode,
                    !it.xattrs.is_empty(),
                    !it.acl.is_empty(),
                ),
                owner,
                group,
                raw_size: it.raw_size.unwrap_or_default(),
                size: it.compressed_size,
                encryption: &it.encryption,
                compression: &it.compression,
                created: it
                    .created
                    .map_or_else(String::new, |d| datetime(TimeFormat::Long, d)),
                modified: it
                    .modified
                    .map_or_else(String::new, |d| datetime(TimeFormat::Long, d)),
                accessed: it
                    .accessed
                    .map_or_else(String::new, |d| datetime(TimeFormat::Long, d)),
                acl: it
                    .acl
                    .iter()
                    .map(|(platform, ace)| AclEntry {
                        platform: platform.to_string(),
                        entries: ace.iter().map(|it| it.to_string()).collect(),
                    })
                    .collect(),
                xattr: it
                    .xattrs
                    .iter()
                    .map(|x| XAttr {
                        key: x.name(),
                        value: base64::engine::general_purpose::STANDARD.encode(x.value()),
                    })
                    .collect(),
            }
        })
        .collect::<Vec<_>>();

    for line in entries {
        serde_json::to_writer(&mut out, &line)?;
        out.write_all(b"\n")?;
    }
    Ok(())
}

fn csv_entries_to(
    entries: Vec<TableRow>,
    options: &ListOptions,
    out: impl Write,
) -> io::Result<()> {
    delimited_entries_to(entries, options, csv::Writer::from_writer(out))
}

fn tsv_entries_to(
    entries: Vec<TableRow>,
    options: &ListOptions,
    out: impl Write,
) -> io::Result<()> {
    delimited_entries_to(
        entries,
        options,
        csv::WriterBuilder::new().delimiter(b'\t').from_writer(out),
    )
}

fn delimited_entries_to(
    entries: Vec<TableRow>,
    options: &ListOptions,
    mut wtr: csv::Writer<impl Write>,
) -> io::Result<()> {
    wtr.write_record([
        "filename",
        "permissions",
        "owner",
        "group",
        "raw_size",
        "compressed_size",
        "encryption",
        "compression",
        options.time_field.as_str(),
    ])?;

    let rows = entries
        .par_iter()
        .map(|row| {
            let permission_mode = row.permission_mode();
            let owner = row.permission.as_ref().map_or_else(String::new, |it| {
                it.owner_display(options.numeric_owner).to_string()
            });
            let group = row.permission.as_ref().map_or_else(String::new, |it| {
                it.group_display(options.numeric_owner).to_string()
            });
            let time = match options.time_field {
                TimeField::Created => row.created,
                TimeField::Modified => row.modified,
                TimeField::Accessed => row.accessed,
            }
            .map_or_else(String::new, |d| datetime(TimeFormat::Long, d));

            [
                row.entry_type.name().to_string(),
                permission_string(
                    &row.entry_type,
                    permission_mode,
                    !row.xattrs.is_empty(),
                    !row.acl.is_empty(),
                ),
                owner,
                group,
                row.raw_size.unwrap_or(0).to_string(),
                row.compressed_size.to_string(),
                row.encryption.clone(),
                row.compression.clone(),
                time,
            ]
        })
        .collect::<Vec<_>>();

    for row in rows {
        wtr.write_record(row)?;
    }

    wtr.flush()?;
    Ok(())
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
struct TreeEntry<'s> {
    name: &'s str,
    kind: DataKind,
}

impl<'s> TreeEntry<'s> {
    #[inline]
    const fn new(name: &'s str, kind: DataKind) -> Self {
        Self { name, kind }
    }
}

fn tree_entries_to(
    entries: Vec<TableRow>,
    options: &ListOptions,
    mut out: impl Write,
) -> io::Result<()> {
    let entries = entries.iter().map(|it| match &it.entry_type {
        EntryType::File(name) => (name.as_str(), DataKind::File),
        EntryType::Directory(name) => (name.as_str(), DataKind::Directory),
        EntryType::SymbolicLink(name, _) => (name.as_str(), DataKind::SymbolicLink),
        EntryType::HardLink(name, _) => (name.as_str(), DataKind::HardLink),
        EntryType::BlockDevice(name) => (name.as_str(), DataKind::BlockDevice),
        EntryType::CharDevice(name) => (name.as_str(), DataKind::CharDevice),
        EntryType::Fifo(name) => (name.as_str(), DataKind::Fifo),
    });
    let map = build_tree_map(entries);
    let tree = build_term_tree(&map, Cow::Borrowed(""), None, DataKind::Directory, options);
    writeln!(out, "{tree}")
}

fn build_tree_map<'s>(
    paths: impl IntoIterator<Item = (&'s str, DataKind)>,
) -> HashMap<&'s str, BTreeSet<TreeEntry<'s>>> {
    let mut tree: HashMap<_, BTreeSet<_>> = HashMap::new();

    for (path, kind) in paths {
        let indices = path
            .char_indices()
            .filter(|(_, c)| *c == '/')
            .map(|(idx, _)| (idx, DataKind::Directory))
            .chain([(path.len(), kind)]);
        let mut start = 0;
        for (end, k) in indices {
            let key = &path[..start];
            let value = &path[start..end];
            let value = value.strip_prefix('/').unwrap_or(value);
            tree.entry(key)
                .or_default()
                .insert(TreeEntry::new(value, k));
            start = end;
        }
    }
    tree
}

fn build_term_tree<'a>(
    tree: &HashMap<&'a str, BTreeSet<TreeEntry<'a>>>,
    root: Cow<'a, str>,
    name: Option<&'a str>,
    kind: DataKind,
    options: &ListOptions,
) -> termtree::Tree<Cow<'a, str>> {
    let label = match name {
        None => Cow::Borrowed("."),
        Some(n) => format_name(n, kind, options),
    };
    let mut node = termtree::Tree::new(label);
    if let Some(children) = tree.get(root.as_ref()) {
        for entry in children {
            let child_root = if root.is_empty() {
                Cow::Borrowed(entry.name)
            } else {
                Cow::Owned(format!("{}/{}", root, entry.name))
            };
            node.push(build_term_tree(
                tree,
                child_root,
                Some(entry.name),
                entry.kind,
                options,
            ));
        }
    }
    node
}

fn format_name<'a>(name: &'a str, kind: DataKind, options: &ListOptions) -> Cow<'a, str> {
    let name = match kind {
        DataKind::Directory if options.classify => Cow::Owned(format!("{name}/")),
        DataKind::SymbolicLink if options.classify => Cow::Owned(format!("{name}@")),
        _ => Cow::Borrowed(name),
    };
    if options.hide_control_chars {
        Cow::Owned(hide_control_chars(&name).to_string())
    } else {
        name
    }
}
