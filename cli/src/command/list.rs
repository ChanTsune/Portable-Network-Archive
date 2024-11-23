#[cfg(feature = "memmap")]
use crate::command::commons::run_read_entries_mem;
#[cfg(not(feature = "memmap"))]
use crate::command::commons::PathArchiveProvider;
use crate::{
    chunk,
    cli::{FileArgs, PasswordArgs},
    command::{
        ask_password,
        commons::{run_read_entries, ArchiveProvider},
        Command,
    },
    ext::*,
    utils::GlobPatterns,
};
use chrono::{DateTime, Local};
use clap::{
    builder::styling::{AnsiColor, Color as Colour, Style},
    ArgGroup, Parser,
};
use pna::{
    prelude::*, Compression, DataKind, Encryption, ExtendedAttribute, NormalEntry, Permission,
    RawChunk, ReadEntry, ReadOptions, SolidHeader,
};
use rayon::prelude::*;
#[cfg(feature = "memmap")]
use std::path::Path;
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    io,
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

struct TableRow {
    encryption: String,
    compression: String,
    permissions: String,
    raw_size: Option<u128>,
    compressed_size: usize,
    user: Option<Subject>,
    group: Option<Subject>,
    created: Option<Duration>,
    modified: Option<Duration>,
    name: String,
    xattrs: Vec<ExtendedAttribute>,
    acl: HashMap<chunk::AcePlatform, Vec<chunk::Ace>>,
    privates: Vec<RawChunk>,
}

struct Subject {
    id: u64,
    name: String,
}

impl Subject {
    #[inline]
    fn value(self, numeric: bool) -> String {
        if numeric {
            self.id.to_string()
        } else {
            self.name
        }
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
        let has_acl = !acl.is_empty();
        let has_xattr = !entry.xattrs().is_empty();
        Ok(Self {
            encryption: match solid
                .map(|s| (s.encryption(), s.cipher_mode()))
                .unwrap_or_else(|| (header.encryption(), header.cipher_mode()))
            {
                (Encryption::No, _) => "-".into(),
                (encryption, cipher_mode) => {
                    format!("{:?}({:?})", encryption, cipher_mode).to_ascii_lowercase()
                }
            },
            compression: match (
                solid
                    .map(|s| s.compression())
                    .unwrap_or(header.compression()),
                solid,
            ) {
                (Compression::No, None) => "-".into(),
                (Compression::No, Some(_)) => "-(solid)".into(),
                (method, None) => format!("{:?}", method).to_ascii_lowercase(),
                (method, Some(_)) => format!("{:?}(solid)", method).to_ascii_lowercase(),
            },
            permissions: paint_permission(
                header.data_kind(),
                metadata.permission(),
                has_xattr,
                has_acl,
            ),
            raw_size: metadata.raw_file_size(),
            compressed_size: metadata.compressed_size(),
            user: metadata.permission().map(|p| Subject {
                id: p.uid(),
                name: p.uname().into(),
            }),
            group: metadata.permission().map(|p| Subject {
                id: p.gid(),
                name: p.gname().into(),
            }),
            created: metadata.created(),
            modified: metadata.modified(),
            name: if matches!(
                header.data_kind(),
                DataKind::SymbolicLink | DataKind::HardLink
            ) {
                let original = entry
                    .reader(ReadOptions::with_password(password))
                    .map(|r| io::read_to_string(r).unwrap_or_else(|_| "-".into()))
                    .unwrap_or_default();
                format!("{} -> {}", header.path(), original)
            } else {
                header.path().to_string()
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
        numeric_owner: args.numeric_owner,
    };
    #[cfg(not(feature = "memmap"))]
    {
        run_list_archive(
            PathArchiveProvider::new(&args.file.archive),
            password.as_deref(),
            &args.file.files,
            options,
        )
    }
    #[cfg(feature = "memmap")]
    {
        run_list_archive_mem(
            &args.file.archive,
            password.as_deref(),
            &args.file.files,
            options,
        )
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum TimeFormat {
    Auto(SystemTime),
    Long,
}

pub(crate) struct ListOptions {
    pub(crate) long: bool,
    pub(crate) header: bool,
    pub(crate) solid: bool,
    pub(crate) show_xattr: bool,
    pub(crate) show_acl: bool,
    pub(crate) show_private: bool,
    pub(crate) time_format: TimeFormat,
    pub(crate) numeric_owner: bool,
}

pub(crate) fn run_list_archive(
    archive_provider: impl ArchiveProvider,
    password: Option<&str>,
    files: &[String],
    args: ListOptions,
) -> io::Result<()> {
    let globs =
        GlobPatterns::new(files).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

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
    print_entries(entries, globs, args);
    Ok(())
}

#[cfg(feature = "memmap")]
pub(crate) fn run_list_archive_mem(
    archive_provider: impl AsRef<Path>,
    password: Option<&str>,
    files: &[String],
    args: ListOptions,
) -> io::Result<()> {
    let globs =
        GlobPatterns::new(files).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let mut entries = Vec::new();

    run_read_entries_mem(archive_provider, |entry| {
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
    print_entries(entries, globs, args);
    Ok(())
}

fn print_entries(entries: Vec<TableRow>, globs: GlobPatterns, options: ListOptions) {
    if entries.is_empty() {
        return;
    }

    let entries = if globs.is_empty() {
        entries
    } else {
        entries
            .into_par_iter()
            .filter(|r| globs.matches_any(&r.name))
            .collect()
    };
    if options.long {
        detail_list_entries(entries.into_iter(), options);
    } else {
        simple_list_entries(entries.into_iter());
    }
}

fn simple_list_entries(entries: impl Iterator<Item = TableRow>) {
    for path in entries {
        println!("{}", path.name)
    }
}

fn detail_list_entries(entries: impl Iterator<Item = TableRow>, options: ListOptions) {
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
        "Created",
        "Modified",
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
        builder.push_record([
            content.encryption,
            content.compression,
            content.permissions,
            content
                .raw_size
                .map_or_else(|| "-".into(), |size| size.to_string()),
            content.compressed_size.to_string(),
            content
                .user
                .map_or_else(|| "-".into(), |it| it.value(options.numeric_owner)),
            content
                .group
                .map_or_else(|| "-".into(), |it| it.value(options.numeric_owner)),
            datetime(options.time_format, content.created),
            datetime(options.time_format, content.modified),
            content.name,
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
    println!("{}", table);
}

const DURATION_SIX_MONTH: Duration = Duration::from_secs(60 * 60 * 24 * 30 * 6);

fn within_six_months(now: SystemTime, x: SystemTime) -> bool {
    let six_months_ago = now - DURATION_SIX_MONTH;
    six_months_ago <= x
}

fn datetime(format: TimeFormat, d: Option<Duration>) -> String {
    match d {
        None => "-".into(),
        Some(d) => {
            let time = UNIX_EPOCH + d;
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
    }
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
    fn paint(&self, v: T) -> StyledDisplay<T> {
        StyledDisplay { style: self, v }
    }
}

const STYLE_READ: Style = Style::new().fg_color(Some(Colour::Ansi(AnsiColor::Yellow)));
const STYLE_WRITE: Style = Style::new().fg_color(Some(Colour::Ansi(AnsiColor::Red)));
const STYLE_EXEC: Style = Style::new().fg_color(Some(Colour::Ansi(AnsiColor::Blue)));
const STYLE_DIR: Style = Style::new().fg_color(Some(Colour::Ansi(AnsiColor::Magenta)));
const STYLE_LINK: Style = Style::new().fg_color(Some(Colour::Ansi(AnsiColor::Cyan)));
const STYLE_HYPHEN: Style = Style::new();

fn kind_paint(kind: DataKind) -> impl Display + 'static {
    match kind {
        DataKind::File | DataKind::HardLink => STYLE_HYPHEN.paint('.'),
        DataKind::Directory => STYLE_DIR.paint('d'),
        DataKind::SymbolicLink => STYLE_LINK.paint('l'),
    }
}

fn paint_permission(
    kind: DataKind,
    permission: Option<&Permission>,
    has_xattr: bool,
    has_acl: bool,
) -> String {
    let permission = permission.map(|p| p.permissions()).unwrap_or_default();
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
