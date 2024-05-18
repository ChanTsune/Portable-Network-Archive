use crate::{
    chunk,
    cli::{FileArgs, PasswordArgs, Verbosity},
    command::{
        ask_password,
        commons::{run_across_archive, ArchiveProvider, PathArchiveProvider},
        Command,
    },
    utils::GlobPatterns,
};
use ansi_term::{ANSIString, Colour, Style};
use chrono::{DateTime, Local};
use clap::{ArgGroup, Parser};
use pna::{
    Chunk, Compression, DataKind, Encryption, ExtendedAttribute, ReadEntry, ReadOption,
    RegularEntry, SolidHeader,
};
use rayon::prelude::*;
use std::{
    io,
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tabled::{
    builder::Builder as TableBuilder,
    settings::{
        object::{Rows, Segment},
        themes::Colorization,
        Alignment, Color, Modify, Padding, Style as TableStyle,
    },
};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[clap(disable_help_flag = true)]
#[command(
    group(ArgGroup::new("unstable-acl").args(["show_acl"]).requires("unstable")),
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
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        list_archive(self, verbosity)
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
}

impl TableRow {
    fn from_xattr(name: &str, value: &[u8]) -> Self {
        Self {
            encryption: String::new(),
            compression: String::new(),
            permissions: name.to_owned(),
            raw_size: value.len().to_string(),
            compressed_size: String::new(),
            user: String::new(),
            group: String::new(),
            created: String::new(),
            modified: String::new(),
            name: String::new(),
        }
    }

    fn from_acl(acl: chunk::Ace) -> Self {
        Self {
            encryption: String::new(),
            compression: String::new(),
            permissions: acl.to_string(),
            raw_size: String::new(),
            compressed_size: String::new(),
            user: String::new(),
            group: String::new(),
            created: String::new(),
            modified: String::new(),
            name: String::new(),
        }
    }
}

impl
    From<(
        RegularEntry,
        Option<&str>,
        SystemTime,
        Option<&SolidHeader>,
        bool,
    )> for TableRow
{
    fn from(
        (entry, password, now, solid, numeric_owner): (
            RegularEntry,
            Option<&str>,
            SystemTime,
            Option<&SolidHeader>,
            bool,
        ),
    ) -> Self {
        let header = entry.header();
        let metadata = entry.metadata();
        TableRow {
            encryption: match solid.map(|s| s.encryption()).unwrap_or(header.encryption()) {
                Encryption::No => "-".to_string(),
                _ => format!("{:?}({:?})", header.encryption(), header.cipher_mode())
                    .to_ascii_lowercase(),
            },
            compression: match (
                solid
                    .map(|s| s.compression())
                    .unwrap_or(header.compression()),
                solid,
            ) {
                (Compression::No, None) => "-".to_string(),
                (Compression::No, Some(_)) => "-(solid)".to_string(),
                (method, None) => format!("{:?}", method).to_ascii_lowercase(),
                (method, Some(_)) => format!("{:?}(solid)", method).to_ascii_lowercase(),
            },
            permissions: metadata
                .permission()
                .map(|p| paint_permission(header.data_kind(), p.permissions(), entry.xattrs()))
                .unwrap_or_else(|| paint_data_kind(header.data_kind(), entry.xattrs()))
                .iter()
                .map(|it| it.to_string())
                .collect::<String>(),
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
                        p.uname().to_string()
                    }
                })
                .unwrap_or_else(|| "-".to_string()),
            group: metadata
                .permission()
                .map(|p| {
                    if numeric_owner {
                        p.gid().to_string()
                    } else {
                        p.gname().to_string()
                    }
                })
                .unwrap_or_else(|| "-".to_string()),
            created: datetime(now, metadata.created()),
            modified: datetime(now, metadata.modified()),
            name: if matches!(
                header.data_kind(),
                DataKind::SymbolicLink | DataKind::HardLink
            ) {
                let path = header.path().to_string();
                let original = entry
                    .reader(ReadOption::with_password(password))
                    .map(|r| io::read_to_string(r).unwrap_or_else(|_| "-".to_string()))
                    .unwrap_or_default();
                format!("{} -> {}", path, original)
            } else {
                header.path().to_string()
            },
        }
    }
}

fn list_archive(args: ListCommand, _: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    run_list_archive(
        PathArchiveProvider::new(&args.file.archive),
        password.as_deref(),
        &args.file.files,
        ListOptions {
            long: args.long,
            header: args.header,
            solid: args.solid,
            show_xattr: args.show_xattr,
            show_acl: args.show_acl,
            numeric_owner: args.numeric_owner,
        },
    )
}

pub(crate) struct ListOptions {
    pub(crate) long: bool,
    pub(crate) header: bool,
    pub(crate) solid: bool,
    pub(crate) show_xattr: bool,
    pub(crate) show_acl: bool,
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
    let mut entries = Vec::<TableRow>::new();

    run_across_archive(archive_provider, |reader| {
        for entry in reader.entries() {
            match entry? {
                ReadEntry::Solid(solid) => {
                    if args.solid {
                        for entry in solid.entries(password)? {
                            let entry = entry?;
                            let xattrs = if args.show_xattr {
                                entry
                                    .xattrs()
                                    .iter()
                                    .map(|xattr| TableRow::from_xattr(xattr.name(), xattr.value()))
                                    .collect::<Vec<_>>()
                            } else {
                                Vec::new()
                            };
                            let acl = if args.show_acl {
                                let mut acl = Vec::new();
                                for c in entry.extra_chunks() {
                                    if c.ty() == chunk::faCe {
                                        let body = std::str::from_utf8(c.data())
                                            .map_err(io::Error::other)?;
                                        let ace =
                                            chunk::Ace::from_str(body).map_err(io::Error::other)?;
                                        acl.push(ace);
                                    }
                                }
                                acl
                            } else {
                                Vec::new()
                            };
                            entries.push(
                                (
                                    entry,
                                    password,
                                    now,
                                    Some(solid.header()),
                                    args.numeric_owner,
                                )
                                    .into(),
                            );
                            for ace in acl {
                                entries.push(TableRow::from_acl(ace));
                            }
                            entries.extend(xattrs);
                        }
                    } else {
                        eprintln!("warning: this archive contain solid mode entry. if you need to show it use --solid option.");
                    }
                }
                ReadEntry::Regular(item) => {
                    let xattrs = if args.show_xattr {
                        item.xattrs()
                            .iter()
                            .map(|xattr| TableRow::from_xattr(xattr.name(), xattr.value()))
                            .collect::<Vec<_>>()
                    } else {
                        Vec::new()
                    };
                    let acl = if args.show_acl {
                        let mut acl = Vec::new();
                        for c in item.extra_chunks() {
                            if c.ty() == chunk::faCe {
                                let body =
                                    std::str::from_utf8(c.data()).map_err(io::Error::other)?;
                                let ace = chunk::Ace::from_str(body).map_err(io::Error::other)?;
                                acl.push(ace);
                            }
                        }
                        acl
                    } else {
                        Vec::new()
                    };
                    entries.push((item, password, now, None, args.numeric_owner).into());
                    for ace in acl {
                        entries.push(TableRow::from_acl(ace));
                    }
                    entries.extend(xattrs);
                }
            }
        }
        Ok(())
    })?;

    if entries.is_empty() {
        return Ok(());
    }

    let entries = if globs.is_empty() {
        entries
    } else {
        entries
            .into_par_iter()
            .filter(|r| globs.matches_any_path(r.name.as_ref()))
            .collect()
    };
    if args.long {
        detail_list_entries(entries.into_iter(), args.header);
    } else {
        simple_list_entries(entries.into_iter());
    }
    Ok(())
}

fn simple_list_entries(entries: impl Iterator<Item = TableRow>) {
    for path in entries {
        println!("{}", path.name)
    }
}

fn detail_list_entries(entries: impl Iterator<Item = TableRow>, print_header: bool) {
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

    let mut builder = TableBuilder::new();
    if print_header {
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
    if print_header {
        table.with(Colorization::exact([underline], Rows::first()));
    }
    table.with(Padding::new(0, 1, 0, 0).colorize(
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
        None => "-".to_string(),
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

fn paint_data_kind(kind: DataKind, xattrs: &[ExtendedAttribute]) -> Vec<ANSIString<'static>> {
    let style_dir = Style::new().fg(Colour::Purple);
    let style_link = Style::new().fg(Colour::Cyan);
    let style_hyphen = Style::new();
    vec![
        kind_paint(kind, style_hyphen, style_dir, style_link),
        style_hyphen.paint("_"),
        style_hyphen.paint("_"),
        style_hyphen.paint("_"),
        style_hyphen.paint("_"),
        style_hyphen.paint("_"),
        style_hyphen.paint("_"),
        style_hyphen.paint("_"),
        style_hyphen.paint("_"),
        style_hyphen.paint("_"),
        style_hyphen.paint(if xattrs.is_empty() { " " } else { "@" }),
    ]
}

fn kind_paint(
    kind: DataKind,
    style_hyphen: Style,
    style_dir: Style,
    style_link: Style,
) -> ANSIString<'static> {
    match kind {
        DataKind::File | DataKind::HardLink => style_hyphen.paint("."),
        DataKind::Directory => style_dir.paint("d"),
        DataKind::SymbolicLink => style_link.paint("l"),
    }
}

fn paint_permission(
    kind: DataKind,
    permission: u16,
    xattrs: &[ExtendedAttribute],
) -> Vec<ANSIString<'static>> {
    let style_read = Style::new().fg(Colour::Yellow);
    let style_write = Style::new().fg(Colour::Red);
    let style_exec = Style::new().fg(Colour::Blue);
    let style_dir = Style::new().fg(Colour::Purple);
    let style_link = Style::new().fg(Colour::Cyan);
    let style_hyphen = Style::new();

    let style_paint = |style: Style, c: &'static str, h: &'static str, bool: bool| {
        if bool {
            style.paint(c)
        } else {
            style_hyphen.paint(h)
        }
    };
    let paint =
        |style: Style, c: &'static str, bit: u16| style_paint(style, c, "-", permission & bit != 0);

    vec![
        kind_paint(kind, style_hyphen, style_dir, style_link),
        paint(style_read, "r", 0b100000000),  // owner_read
        paint(style_write, "w", 0b010000000), // owner_write
        paint(style_exec, "x", 0b001000000),  // owner_exec
        paint(style_read, "r", 0b000100000),  // group_read
        paint(style_write, "w", 0b000010000), // group_write
        paint(style_exec, "x", 0b000001000),  // group_exec
        paint(style_read, "r", 0b000000100),  // other_read
        paint(style_write, "w", 0b000000010), // other_write
        paint(style_exec, "x", 0b000000001),  // other_exec
        style_hyphen.paint(if xattrs.is_empty() { " " } else { "@" }),
    ]
}
