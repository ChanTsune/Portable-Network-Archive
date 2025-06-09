#[cfg(feature = "memmap")]
use crate::command::commons::run_read_entries_mem;
use crate::{
    chunk,
    cli::{FileArgs, PasswordArgs},
    command::{
        ask_password,
        commons::{collect_split_archives, run_read_entries, Exclude},
        Command,
    },
    ext::*,
    utils::{self, GlobPatterns},
};
use base64::Engine;
use chrono::{DateTime, Local};
use clap::{
    builder::styling::{AnsiColor, Color as Colour, Style},
    ArgGroup, Parser, ValueHint,
};
use pna::{
    prelude::*, Compression, DataKind, Encryption, ExtendedAttribute, NormalEntry, RawChunk,
    ReadEntry, ReadOptions, SolidHeader,
};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    fmt::{self, Display, Formatter},
    io::{self, prelude::*},
    path::PathBuf,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tabled::{
    builder::Builder as TableBuilder,
    settings::{
        object::{Rows, Segment},
        themes::Colorization,
        Alignment, Color, Modify, Padding, PaddingColor, Style as TableStyle,
    },
};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[clap(disable_help_flag = true)]
#[command(
    group(ArgGroup::new("unstable-acl").args(["show_acl"]).requires("unstable")),
    group(ArgGroup::new("unstable-private-chunk").args(["show_private"]).requires("unstable")),
    group(ArgGroup::new("unstable-format").args(["format"]).requires("unstable")),
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
    #[arg(short = 'e', help = "Display acl in a table (unstable)")]
    pub(crate) show_acl: bool,
    #[arg(
        long = "private",
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
    #[arg(long, help = "Display format")]
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
    #[arg(long, help = "Exclude path glob (unstable)", value_hint = ValueHint::AnyPath)]
    exclude: Option<Vec<String>>,
    #[arg(long, help = "Read exclude files from given path (unstable)", value_hint = ValueHint::FilePath)]
    exclude_from: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) file: FileArgs,
    #[arg(long, action = clap::ArgAction::Help)]
    help: Option<bool>,
}

impl Command for ListCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        list_archive(self)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Format {
    Table,
    JsonL,
    Tree,
}

impl FromStr for Format {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "table" => Ok(Self::Table),
            "jsonl" => Ok(Self::JsonL),
            "tree" => Ok(Self::Tree),
            unknown => Err(format!("unknown value: {unknown}")),
        }
    }
}

#[derive(Debug)]
enum EntryType {
    File(String),
    Directory(String),
    SymbolicLink(String, String),
    HardLink(String, String),
}

impl EntryType {
    #[inline]
    fn name(&self) -> &str {
        match self {
            EntryType::File(name)
            | EntryType::Directory(name)
            | EntryType::SymbolicLink(name, _)
            | EntryType::HardLink(name, _) => name,
        }
    }
}

struct TableRow {
    encryption: String,
    compression: String,
    permission: Option<pna::Permission>,
    raw_size: Option<u128>,
    compressed_size: usize,
    created: Option<Duration>,
    modified: Option<Duration>,
    accessed: Option<Duration>,
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

impl<T> TryFrom<(&NormalEntry<T>, Option<&str>, Option<&SolidHeader>)> for TableRow
where
    T: AsRef<[u8]> + Clone,
    RawChunk<T>: Chunk,
    RawChunk: From<RawChunk<T>>,
{
    type Error = io::Error;
    #[inline]
    fn try_from(
        (entry, password, solid): (&NormalEntry<T>, Option<&str>, Option<&SolidHeader>),
    ) -> Result<Self, Self::Error> {
        let header = entry.header();
        let metadata = entry.metadata();
        let acl = entry.acl()?;
        Ok(Self {
            encryption: match solid.map_or_else(
                || (header.encryption(), header.cipher_mode()),
                |s| (s.encryption(), s.cipher_mode()),
            ) {
                (Encryption::No, _) => "-".into(),
                (encryption, cipher_mode) => {
                    format!("{encryption:?}({cipher_mode:?})").to_ascii_lowercase()
                }
            },
            compression: match (
                solid.map_or(header.compression(), |s| s.compression()),
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
            created: metadata.created(),
            modified: metadata.modified(),
            accessed: metadata.accessed(),
            entry_type: match header.data_kind() {
                DataKind::SymbolicLink => EntryType::SymbolicLink(
                    header.path().to_string(),
                    entry
                        .reader(ReadOptions::with_password(password))
                        .and_then(io::read_to_string)
                        .unwrap_or_else(|_| "-".into()),
                ),
                DataKind::HardLink => EntryType::HardLink(
                    header.path().to_string(),
                    entry
                        .reader(ReadOptions::with_password(password))
                        .and_then(io::read_to_string)
                        .unwrap_or_else(|_| "-".into()),
                ),
                DataKind::Directory => EntryType::Directory(header.path().to_string()),
                DataKind::File => EntryType::File(header.path().to_string()),
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

fn list_archive(args: ListCommand) -> io::Result<()> {
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
    };
    let files_globs = GlobPatterns::new(&args.file.files)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let exclude = {
        let mut exclude = args.exclude.unwrap_or_default();
        if let Some(p) = args.exclude_from {
            exclude.extend(utils::fs::read_to_lines(p)?);
        }
        Exclude {
            include: args.include.unwrap_or_default().into(),
            exclude: exclude.into(),
        }
    };

    let archives = collect_split_archives(&args.file.archive)?;

    #[cfg(not(feature = "memmap"))]
    {
        run_list_archive(archives, password.as_deref(), files_globs, exclude, options)
    }
    #[cfg(feature = "memmap")]
    {
        run_list_archive_mem(archives, password.as_deref(), files_globs, exclude, options)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum TimeFormat {
    Auto(SystemTime),
    Long,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
            TimeField::Created => "created",
            TimeField::Modified => "modified",
            TimeField::Accessed => "accessed",
        }
    }
}

impl FromStr for TimeField {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "created" => Ok(Self::Created),
            "modified" => Ok(Self::Modified),
            "accessed" => Ok(Self::Accessed),
            _ => Err(s.into()),
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
}

pub(crate) fn run_list_archive(
    archive_provider: impl IntoIterator<Item = impl Read>,
    password: Option<&str>,
    files_globs: GlobPatterns,
    exclude: Exclude,
    args: ListOptions,
) -> io::Result<()> {
    let mut entries = Vec::new();

    run_read_entries(archive_provider, |entry| {
        match entry? {
            ReadEntry::Solid(solid) if args.solid => {
                for entry in solid.entries(password)? {
                    entries.push((&entry?, password, Some(solid.header())).try_into()?)
                }
            }
            ReadEntry::Solid(_) => {
                log::warn!("This archive contain solid mode entry. if you need to show it use --solid option.");
            }
            ReadEntry::Normal(item) => entries.push((&item, password, None).try_into()?),
        }
        Ok(())
    })?;
    print_entries(entries, files_globs, exclude, args);
    Ok(())
}

#[cfg(feature = "memmap")]
pub(crate) fn run_list_archive_mem(
    archives: Vec<std::fs::File>,
    password: Option<&str>,
    files_globs: GlobPatterns,
    exclude: Exclude,
    args: ListOptions,
) -> io::Result<()> {
    let mut entries = Vec::new();

    run_read_entries_mem(archives, |entry| {
        match entry? {
            ReadEntry::Solid(solid) if args.solid => {
                for entry in solid.entries(password)? {
                    entries.push((&entry?, password, Some(solid.header())).try_into()?);
                }
            }
            ReadEntry::Solid(_) => {
                log::warn!("This archive contain solid mode entry. if you need to show it use --solid option.");
            }
            ReadEntry::Normal(item) => entries.push((&item, password, None).try_into()?),
        }
        Ok(())
    })?;
    print_entries(entries, files_globs, exclude, args);
    Ok(())
}

fn print_entries(
    entries: Vec<TableRow>,
    globs: GlobPatterns,
    exclude: Exclude,
    options: ListOptions,
) {
    if entries.is_empty() {
        return;
    }

    let entries = entries
        .into_par_iter()
        .filter(|r| globs.is_empty() || globs.matches_any(r.entry_type.name()))
        .filter(|r| !exclude.excluded(r.entry_type.name()))
        .collect::<Vec<_>>();
    match options.format {
        Some(Format::JsonL) => json_line_entries(entries),
        Some(Format::Table) => detail_list_entries(entries, options),
        Some(Format::Tree) => tree_entries(entries, options),
        None if options.long => detail_list_entries(entries, options),
        None => simple_list_entries(entries, options),
    }
}

fn simple_list_entries(entries: impl IntoParallelIterator<Item = TableRow>, options: ListOptions) {
    let entries = entries
        .into_par_iter()
        .map(|path| {
            let path = match path.entry_type {
                EntryType::Directory(name) if options.classify => format!("{name}/"),
                EntryType::SymbolicLink(name, _) if options.classify => {
                    format!("{name}@")
                }
                EntryType::File(name)
                | EntryType::Directory(name)
                | EntryType::SymbolicLink(name, _)
                | EntryType::HardLink(name, _) => name,
            };
            if options.hide_control_chars {
                hide_control_chars(&path)
            } else {
                path
            }
        })
        .collect::<Vec<_>>();
    for path in entries {
        println!("{path}")
    }
}

fn detail_list_entries(entries: impl IntoIterator<Item = TableRow>, options: ListOptions) {
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
            |it| it.owner_display(options.numeric_owner).to_string(),
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
                    EntryType::File(path) | EntryType::Directory(path) => path,
                    EntryType::SymbolicLink(path, link_to) | EntryType::HardLink(path, link_to) => {
                        format!("{path} -> {link_to}")
                    }
                };
                if options.hide_control_chars {
                    hide_control_chars(&name)
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
    println!("{table}");
}

const DURATION_SIX_MONTH: Duration = Duration::from_secs(60 * 60 * 24 * 30 * 6);

fn within_six_months(now: SystemTime, x: SystemTime) -> bool {
    let six_months_ago = now - DURATION_SIX_MONTH;
    six_months_ago <= x
}

fn datetime(format: TimeFormat, since_unix_epoch: Duration) -> String {
    let time = UNIX_EPOCH + since_unix_epoch;
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
fn hide_control_chars(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
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
    fn paint(&self, v: T) -> StyledDisplay<T>;
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

fn kind_char(kind: &EntryType) -> char {
    match kind {
        EntryType::File(_) | EntryType::HardLink(_, _) => '.',
        EntryType::Directory(_) => 'd',
        EntryType::SymbolicLink(_, _) => 'l',
    }
}

fn permission_string(kind: &EntryType, permission: u16, has_xattr: bool, has_acl: bool) -> String {
    #[inline(always)]
    fn paint(permission: u16, c: char, bit: u16) -> char {
        if permission & bit != 0 {
            c
        } else {
            '-'
        }
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

#[derive(Serialize, Deserialize, Debug)]
struct FileInfo {
    filename: String,
    permissions: String,
    owner: String,
    group: String,
    raw_size: u128,
    size: usize,
    encryption: String,
    compression: String,
    created: String,
    modified: String,
    accessed: String,
    acl: Vec<AclEntry>,
    xattr: Vec<XAttr>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AclEntry {
    platform: String,
    entries: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct XAttr {
    key: String,
    value: String,
}

fn json_line_entries(entries: impl IntoParallelIterator<Item = TableRow>) {
    let entries = entries
        .into_par_iter()
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
                filename: it.entry_type.name().into(),
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
                encryption: it.encryption,
                compression: it.compression,
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
                    .into_iter()
                    .map(|(platform, ace)| AclEntry {
                        platform: platform.to_string(),
                        entries: ace.into_iter().map(|it| it.to_string()).collect(),
                    })
                    .collect(),
                xattr: it
                    .xattrs
                    .into_iter()
                    .map(|x| XAttr {
                        key: x.name().into(),
                        value: base64::engine::general_purpose::STANDARD.encode(x.value()),
                    })
                    .collect(),
            }
        })
        .collect::<Vec<_>>();

    print!("{}", JsonLDisplay(entries));
}

struct JsonLDisplay<T>(Vec<T>);

impl<T: serde::Serialize> Display for JsonLDisplay<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        struct FormatterAdapter<'f, 'a>(&'f mut Formatter<'a>);
        impl Write for FormatterAdapter<'_, '_> {
            #[inline]
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0
                    .write_str(unsafe { std::str::from_utf8_unchecked(buf) })
                    .map_err(io::Error::other)?;
                Ok(buf.len())
            }

            #[inline]
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        impl fmt::Write for FormatterAdapter<'_, '_> {
            #[inline]
            fn write_str(&mut self, s: &str) -> fmt::Result {
                self.0.write_str(s)
            }

            #[inline]
            fn write_char(&mut self, c: char) -> fmt::Result {
                self.0.write_char(c)
            }

            #[inline]
            fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
                self.0.write_fmt(args)
            }
        }
        let mut writer = FormatterAdapter(f);
        for line in &self.0 {
            use core::fmt::Write;
            serde_json::to_writer(&mut writer, &line).map_err(|_| fmt::Error)?;
            writer.write_char('\n')?;
        }
        Ok(())
    }
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

fn tree_entries(entries: impl IntoParallelIterator<Item = TableRow>, options: ListOptions) {
    let entries = entries
        .into_par_iter()
        .map(|it| match it.entry_type {
            EntryType::File(name) => (name, DataKind::File),
            EntryType::Directory(name) => (name, DataKind::Directory),
            EntryType::SymbolicLink(name, _) => (name, DataKind::SymbolicLink),
            EntryType::HardLink(name, _) => (name, DataKind::HardLink),
        })
        .collect::<Vec<_>>();
    let entries = entries
        .par_iter()
        .map(|(name, kind)| (name.as_str(), *kind))
        .collect::<Vec<_>>();
    let tree = build_tree(&entries);
    println!(".");
    display_tree(&tree, "", "", &options);
}

fn build_tree<'s>(paths: &[(&'s str, DataKind)]) -> HashMap<&'s str, BTreeSet<TreeEntry<'s>>> {
    let mut tree: HashMap<_, BTreeSet<_>> = HashMap::new();

    for (path, kind) in paths {
        let indices = path
            .char_indices()
            .filter(|(_, c)| *c == '/')
            .map(|(idx, _)| (idx, DataKind::Directory))
            .chain([(path.len(), *kind)]);
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

fn display_tree(
    tree: &HashMap<&str, BTreeSet<TreeEntry>>,
    root: &str,
    prefix: &str,
    options: &ListOptions,
) {
    if let Some(children) = tree.get(root) {
        for (i, TreeEntry { name: child, kind }) in children.iter().enumerate() {
            let is_last = i == children.len() - 1;
            let branch = if is_last { "└── " } else { "├── " };
            match kind {
                DataKind::Directory if options.classify => {
                    println!("{prefix}{branch}{child}/")
                }
                DataKind::SymbolicLink if options.classify => {
                    println!("{prefix}{branch}{child}@")
                }
                DataKind::File
                | DataKind::Directory
                | DataKind::SymbolicLink
                | DataKind::HardLink => println!("{prefix}{branch}{child}"),
            };

            let new_root = if root.is_empty() {
                Cow::Borrowed(*child)
            } else {
                Cow::Owned(format!("{root}/{child}"))
            };

            let new_prefix = if is_last {
                format!("{prefix}    ")
            } else {
                format!("{prefix}│   ")
            };

            display_tree(tree, &new_root, &new_prefix, options);
        }
    }
}
