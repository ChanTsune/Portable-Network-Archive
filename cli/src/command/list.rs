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
    prelude::*, Compression, DataKind, Encryption, ExtendedAttribute, NormalEntry, RawChunk,
    ReadEntry, ReadOptions, SolidHeader,
};
use rayon::prelude::*;
#[cfg(feature = "memmap")]
use std::path::Path;
use std::{
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
    raw_size: String,
    compressed_size: String,
    user: String,
    group: String,
    created: String,
    modified: String,
    name: String,
    xattrs: Vec<ExtendedAttribute>,
    acl: Vec<chunk::Ace>,
    privates: Vec<RawChunk>,
}

impl<T>
    TryFrom<(
        &NormalEntry<T>,
        Option<&str>,
        SystemTime,
        Option<&SolidHeader>,
        bool,
    )> for TableRow
where
    T: AsRef<[u8]> + Clone,
    RawChunk<T>: Chunk,
    RawChunk: From<RawChunk<T>>,
{
    type Error = io::Error;
    #[inline]
    fn try_from(
        (entry, password, now, solid, numeric_owner): (
            &NormalEntry<T>,
            Option<&str>,
            SystemTime,
            Option<&SolidHeader>,
            bool,
        ),
    ) -> Result<Self, Self::Error> {
        let header = entry.header();
        let metadata = entry.metadata();
        Ok(Self {
            encryption: match solid.map(|s| s.encryption()).unwrap_or(header.encryption()) {
                Encryption::No => "-".into(),
                _ => format!("{:?}({:?})", header.encryption(), header.cipher_mode())
                    .to_ascii_lowercase(),
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
            permissions: metadata
                .permission()
                .map(|p| paint_permission(header.data_kind(), p.permissions(), entry.xattrs()))
                .unwrap_or_else(|| paint_data_kind(header.data_kind(), entry.xattrs())),
            raw_size: metadata
                .raw_file_size()
                .map_or("-".into(), |size| size.to_string()),
            compressed_size: metadata.compressed_size().to_string(),
            user: metadata
                .permission()
                .map(|p| {
                    if numeric_owner {
                        p.uid().to_string()
                    } else {
                        p.uname().into()
                    }
                })
                .unwrap_or_else(|| "-".into()),
            group: metadata
                .permission()
                .map(|p| {
                    if numeric_owner {
                        p.gid().to_string()
                    } else {
                        p.gname().into()
                    }
                })
                .unwrap_or_else(|| "-".into()),
            created: datetime(now, metadata.created()),
            modified: datetime(now, metadata.modified()),
            name: if matches!(
                header.data_kind(),
                DataKind::SymbolicLink | DataKind::HardLink
            ) {
                let path = header.path().to_string();
                let original = entry
                    .reader(ReadOptions::with_password(password))
                    .map(|r| io::read_to_string(r).unwrap_or_else(|_| "-".into()))
                    .unwrap_or_default();
                format!("{} -> {}", path, original)
            } else {
                header.path().to_string()
            },
            xattrs: entry.xattrs().to_vec(),
            acl: entry.acl()?,
            privates: entry
                .extra_chunks()
                .iter()
                .filter(|it| it.ty() != chunk::faCe)
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

pub(crate) struct ListOptions {
    pub(crate) long: bool,
    pub(crate) header: bool,
    pub(crate) solid: bool,
    pub(crate) show_xattr: bool,
    pub(crate) show_acl: bool,
    pub(crate) show_private: bool,
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

    let now = SystemTime::now();
    let mut entries = Vec::new();

    run_read_entries(archive_provider, |entry| {
        match entry? {
            ReadEntry::Solid(solid) if args.solid => {
                for entry in solid.entries(password)? {
                    entries.push(
                        (
                            &entry?,
                            password,
                            now,
                            Some(solid.header()),
                            args.numeric_owner,
                        )
                            .try_into()?,
                    )
                }
            }
            ReadEntry::Solid(_) => {
                log::warn!("This archive contain solid mode entry. if you need to show it use --solid option.");
            }
            ReadEntry::Normal(item) => {
                entries.push((&item, password, now, None, args.numeric_owner).try_into()?)
            }
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

    let now = SystemTime::now();
    let mut entries = Vec::new();

    run_read_entries_mem(archive_provider, |entry| {
        match entry? {
            ReadEntry::Solid(solid) if args.solid => {
                for entry in solid.entries(password)? {
                    entries.push(
                        (
                            &entry?,
                            password,
                            now,
                            Some(solid.header()),
                            args.numeric_owner,
                        )
                            .try_into()?,
                    );
                }
            }
            ReadEntry::Solid(_) => {
                log::warn!("This archive contain solid mode entry. if you need to show it use --solid option.");
            }
            ReadEntry::Normal(item) => {
                entries.push((&item, password, now, None, args.numeric_owner).try_into()?)
            }
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
            content.raw_size,
            content.compressed_size,
            content.user,
            content.group,
            content.created,
            content.modified,
            content.name,
        ]);
        if options.show_acl {
            for a in &content.acl {
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

fn datetime(now: SystemTime, d: Option<Duration>) -> String {
    match d {
        None => "-".into(),
        Some(d) => {
            let time = UNIX_EPOCH + d;
            let datetime = DateTime::<Local>::from(time);
            if within_six_months(now, time) {
                datetime.format("%b %e %H:%M").to_string()
            } else {
                datetime.format("%b %e  %Y").to_string()
            }
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

fn paint_data_kind(kind: DataKind, xattrs: &[ExtendedAttribute]) -> String {
    format!(
        "{}_________{}",
        kind_paint(kind),
        if xattrs.is_empty() { " " } else { "@" }
    )
}

fn kind_paint(kind: DataKind) -> impl Display + 'static {
    match kind {
        DataKind::File | DataKind::HardLink => STYLE_HYPHEN.paint("."),
        DataKind::Directory => STYLE_DIR.paint("d"),
        DataKind::SymbolicLink => STYLE_LINK.paint("l"),
    }
}

fn paint_permission(kind: DataKind, permission: u16, xattrs: &[ExtendedAttribute]) -> String {
    fn style_paint<'s>(
        style: &'s Style,
        c: &'s str,
        h: &'s str,
        bool: bool,
    ) -> StyledDisplay<'s, &'s str> {
        if bool {
            style.paint(c)
        } else {
            STYLE_HYPHEN.paint(h)
        }
    }
    let paint = |style: &'static Style, c: &'static str, bit: u16| {
        style_paint(style, c, "-", permission & bit != 0)
    };

    format!(
        "{}{}{}{}{}{}{}{}{}{}{}",
        kind_paint(kind),
        paint(&STYLE_READ, "r", 0b100000000),  // owner_read
        paint(&STYLE_WRITE, "w", 0b010000000), // owner_write
        paint(&STYLE_EXEC, "x", 0b001000000),  // owner_exec
        paint(&STYLE_READ, "r", 0b000100000),  // group_read
        paint(&STYLE_WRITE, "w", 0b000010000), // group_write
        paint(&STYLE_EXEC, "x", 0b000001000),  // group_exec
        paint(&STYLE_READ, "r", 0b000000100),  // other_read
        paint(&STYLE_WRITE, "w", 0b000000010), // other_write
        paint(&STYLE_EXEC, "x", 0b000000001),  // other_exec
        STYLE_HYPHEN.paint(if xattrs.is_empty() { " " } else { "@" }),
    )
}
