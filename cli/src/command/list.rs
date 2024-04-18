use crate::{
    cli::{FileArgs, PasswordArgs, Verbosity},
    command::{ask_password, Command},
    utils::{part_name, GlobPatterns},
};
use ansi_term::{ANSIString, Colour, Style};
use chrono::{DateTime, Local};
use clap::Parser;
use pna::{
    Archive, Compression, DataKind, Encryption, ExtendedAttribute, ReadOption, RegularEntry,
};
use rayon::prelude::*;
use std::{
    fs::File,
    io,
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
pub(crate) struct ListCommand {
    #[arg(short, long, help = "Display extended file metadata as a table")]
    pub(crate) long: bool,
    #[arg(short, long, help = "Add a header row to each column")]
    pub(crate) header: bool,
    #[arg(long, help = "Display solid mode archive entries")]
    pub(crate) solid: bool,
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

fn list_archive(args: ListCommand, _: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    let globs = GlobPatterns::new(args.file.files.iter().map(|p| p.to_string_lossy()))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let file = File::open(&args.file.archive)?;

    let mut entries = vec![];
    let mut reader = Archive::read_header(file)?;
    let mut num_archive = 1;
    loop {
        if args.solid {
            for entry in reader.entries_with_password(password.as_deref()) {
                let item = entry?;
                entries.push(item);
            }
        } else {
            for entry in reader.entries_skip_solid() {
                let item = entry?;
                entries.push(item);
            }
        };
        if reader.next_archive() {
            num_archive += 1;
            if let Ok(file) = File::open(part_name(&args.file.archive, num_archive).unwrap()) {
                reader = reader.read_next_archive(file)?;
            } else {
                eprintln!("Detected that the file has been split, but the following file could not be found.");
                break;
            }
        } else {
            break;
        }
    }

    if entries.is_empty() {
        return Ok(());
    }

    let entries = if globs.is_empty() {
        entries
    } else {
        entries
            .into_par_iter()
            .filter(|e| globs.matches_any_path(e.header().path().as_ref()))
            .collect()
    };
    if args.long {
        detail_list_entries(entries, password.as_deref(), args.header);
    } else {
        simple_list_entries(&entries);
    }
    Ok(())
}

fn simple_list_entries(entries: &[RegularEntry]) {
    for entry in entries {
        println!("{}", entry.header().path())
    }
}

fn detail_list_entries(entries: Vec<RegularEntry>, password: Option<&str>, print_header: bool) {
    let now = SystemTime::now();
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
    for entry in entries.iter() {
        let header = entry.header();
        let metadata = entry.metadata();
        builder.push_record([
            match header.encryption() {
                Encryption::No => "-".to_string(),
                _ => format!("{:?}({:?})", header.encryption(), header.cipher_mode())
                    .to_ascii_lowercase(),
            },
            match header.compression() {
                Compression::No => "-".to_string(),
                method => format!("{:?}", method).to_ascii_lowercase(),
            },
            metadata
                .permission()
                .map(|p| paint_permission(header.data_kind(), p.permissions(), entry.xattrs()))
                .unwrap_or_else(|| paint_data_kind(header.data_kind(), entry.xattrs()))
                .iter()
                .map(|it| it.to_string())
                .collect::<String>(),
            metadata
                .raw_file_size()
                .map_or("-".into(), |size| size.to_string()),
            metadata.compressed_size().to_string(),
            metadata
                .permission()
                .map(|p| p.uname())
                .unwrap_or("-")
                .to_string(),
            metadata
                .permission()
                .map(|p| p.gname())
                .unwrap_or("-")
                .to_string(),
            datetime(now, metadata.created()),
            datetime(now, metadata.modified()),
            if matches!(
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
