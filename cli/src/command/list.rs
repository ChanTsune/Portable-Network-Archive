mod table;

use crate::{
    cli::{ListArgs, Verbosity},
    command::{
        ask_password,
        list::table::{Cell, Padding, Table, TableRow},
    },
    utils::part_name,
};
use ansi_term::{ANSIString, Colour, Style};
use chrono::{DateTime, Local};
use glob::Pattern;
use libpna::{ArchiveReader, Encryption, EntryHeader, Metadata, ReadEntry};
use std::{
    fs::File,
    io,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(crate) fn list_archive(args: ListArgs, _: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    let globs = args
        .file
        .files
        .iter()
        .map(|p| Pattern::new(&p.to_string_lossy()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let file = File::open(&args.file.archive)?;

    let mut entries = vec![];
    let mut reader = ArchiveReader::read_header(file)?;
    let mut num_archive = 1;
    loop {
        if args.solid {
            for entry in reader.entries_with_password(password.clone()) {
                let item = entry?;
                entries.push((item.header().clone(), item.metadata().clone()));
            }
        } else {
            for entry in reader.entries() {
                let item = entry?;
                entries.push((item.header().clone(), item.metadata().clone()));
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
            .into_iter()
            .filter(|(h, _)| globs.iter().any(|glob| glob.matches(h.path().as_ref())))
            .collect()
    };
    if args.long {
        detail_list_entries(&entries, args.header);
    } else {
        simple_list_entries(&entries);
    }
    Ok(())
}

fn simple_list_entries(entries: &[(EntryHeader, Metadata)]) {
    for (entry, _) in entries {
        println!("{}", entry.path())
    }
}

fn detail_list_entries(entries: &[(EntryHeader, Metadata)], header: bool) {
    let now = SystemTime::now();
    let style_encryption_column = Style::new().fg(Colour::Purple);
    let style_compression_column = Style::new().fg(Colour::Blue);
    let style_compressed_size_column = Style::new().fg(Colour::Green);
    let style_date = Style::new().fg(Colour::Cyan);
    let style_entry = Style::new();
    let mut table = if header {
        let style_header_line = Style::new().underline();
        Table::new_with_header(table::header(style_header_line))
    } else {
        Table::new()
    };
    for (entry, metadata) in entries {
        table.push(TableRow::new([
            Cell::new(
                style_encryption_column,
                match entry.encryption() {
                    Encryption::No => "-".to_string(),
                    _ => format!("{:?}({:?})", entry.encryption(), entry.cipher_mode())
                        .to_ascii_lowercase(),
                },
            ),
            Cell::new(
                style_compression_column,
                format!("{:?}", entry.compression()).to_ascii_lowercase(),
            ),
            Cell::new_text(
                style_date,
                Padding::Right,
                metadata
                    .permission()
                    .map(|p| paint_permission(false, p.permissions()))
                    .unwrap_or(vec![Style::new().paint("-")]),
            ),
            Cell::new_with_pad_direction(
                style_compressed_size_column,
                Padding::Left,
                metadata.compressed_size(),
            ),
            Cell::new(
                style_date,
                metadata.permission().map(|p| p.uname()).unwrap_or("-"),
            ),
            Cell::new(
                style_date,
                metadata.permission().map(|p| p.gname()).unwrap_or("-"),
            ),
            Cell::new(style_date, datetime(now, metadata.created())),
            Cell::new(style_date, datetime(now, metadata.modified())),
            Cell::new(style_entry, entry.path()),
        ]));
    }
    for row in table.into_render_rows() {
        println!("{}", row)
    }
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

fn paint_permission(is_dir: bool, permission: u16) -> Vec<ANSIString<'static>> {
    let style_read = Style::new().fg(Colour::Yellow);
    let style_write = Style::new().fg(Colour::Red);
    let style_exec = Style::new().fg(Colour::Blue);
    let style_dir = Style::new().fg(Colour::Purple);
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
        style_paint(style_dir, "d", ".", is_dir),
        paint(style_read, "r", 0b100000000),  // owner_read
        paint(style_write, "w", 0b010000000), // owner_write
        paint(style_exec, "x", 0b001000000),  // owner_exec
        paint(style_read, "r", 0b000100000),  // group_read
        paint(style_write, "w", 0b000010000), // group_write
        paint(style_exec, "x", 0b000001000),  // group_exec
        paint(style_read, "r", 0b000000100),  // other_read
        paint(style_write, "w", 0b000000010), // other_write
        paint(style_exec, "x", 0b000000001),  // other_exec
    ]
}
