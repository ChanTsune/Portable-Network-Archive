use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs, FileArgs, PasswordArgs, Verbosity},
    command::{
        ask_password, check_password,
        commons::{collect_items, create_entry, entry_option, split_to_parts},
        Command,
    },
    utils::{part_name, Let},
};
use bytesize::ByteSize;
use clap::Parser;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use pna::{Archive, EntryPart, SolidEntryBuilder, WriteOption, MIN_CHUNK_BYTES_SIZE, PNA_HEADER};
use rayon::ThreadPoolBuilder;
use std::{
    fs::{self, File},
    io,
    time::Instant,
};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct CreateCommand {
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    pub(crate) recursive: bool,
    #[arg(long, help = "Overwrite file")]
    pub(crate) overwrite: bool,
    #[arg(long, help = "Archiving the directories")]
    pub(crate) keep_dir: bool,
    #[arg(long, help = "Archiving the timestamp of the files")]
    pub(crate) keep_timestamp: bool,
    #[arg(long, help = "Archiving the permissions of the files")]
    pub(crate) keep_permission: bool,
    #[arg(long, help = "Archiving the extended attributes of the files")]
    pub(crate) keep_xattr: bool,
    #[arg(long, help = "Split archive by total entry size")]
    pub(crate) split: Option<Option<ByteSize>>,
    #[arg(long, help = "Solid mode archive")]
    pub(crate) solid: bool,
    #[command(flatten)]
    pub(crate) compression: CompressionAlgorithmArgs,
    #[command(flatten)]
    pub(crate) cipher: CipherAlgorithmArgs,
    #[command(flatten)]
    pub(crate) password: PasswordArgs,
    #[command(flatten)]
    pub(crate) file: FileArgs,
}

impl Command for CreateCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        create_archive(self, verbosity)
    }
}

fn create_archive(args: CreateCommand, verbosity: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let archive = args.file.archive;
    if !args.overwrite && archive.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is already exists", archive.display()),
        ));
    }
    let start = Instant::now();
    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    if verbosity != Verbosity::Quite {
        eprintln!("Create an archive: {}", archive.display());
    }
    let target_items = collect_items(args.file.files, args.recursive, args.keep_dir)?;

    let progress_bar = if verbosity != Verbosity::Quite {
        Some(
            ProgressBar::new(target_items.len() as u64)
                .with_style(ProgressStyle::default_bar().progress_chars("=> ")),
        )
    } else {
        None
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let cli_option = entry_option(args.compression, args.cipher, password);
    let option = if args.solid {
        WriteOption::store()
    } else {
        cli_option.clone()
    };
    for file in target_items {
        let option = option.clone();
        let keep_timestamp = args.keep_timestamp;
        let keep_permission = args.keep_permission;
        let keep_xattrs = args.keep_xattr;
        let tx = tx.clone();
        pool.spawn_fifo(move || {
            if verbosity == Verbosity::Verbose {
                eprintln!("Adding: {}", file.display());
            }
            tx.send(create_entry(
                &file,
                option,
                keep_timestamp,
                keep_permission,
                keep_xattrs,
            ))
            .unwrap_or_else(|e| panic!("{e}: {}", file.display()));
        });
    }

    drop(tx);

    if let Some(parent) = archive.parent() {
        fs::create_dir_all(parent)?;
    }
    let max_file_size = args
        .split
        .map(|it| it.unwrap_or(ByteSize::gb(1)).0 as usize);

    if args.solid {
        if let Some(max_file_size) = max_file_size {
            let mut entries_builder = SolidEntryBuilder::new(cli_option)?;
            for entry in rx.into_iter() {
                entries_builder.add_entry(entry?)?;
                progress_bar.let_ref(|pb| pb.inc(1));
            }
            let entries = entries_builder.build()?;
            let mut part_num = 1;
            let file = File::create(part_name(&archive, part_num).unwrap())?;
            let mut writer = Archive::write_header(file)?;

            // NOTE: max_file_size - (PNA_HEADER + AHED + ANXT + AEND)
            let max_file_size = max_file_size - (PNA_HEADER.len() + MIN_CHUNK_BYTES_SIZE * 3 + 8);
            let mut written_entry_size = 0;
            let parts = split_to_parts(
                EntryPart::from(entries),
                max_file_size - written_entry_size,
                max_file_size,
            );
            for part in parts {
                if written_entry_size + part.bytes_len() > max_file_size {
                    part_num += 1;
                    let part_n_name = part_name(&archive, part_num).unwrap();
                    if verbosity == Verbosity::Verbose {
                        eprintln!("Split: {} to {}", archive.display(), part_n_name.display());
                    }
                    let file = File::create(&part_n_name)?;
                    writer = writer.split_to_next_archive(file)?;
                    written_entry_size = 0;
                }
                written_entry_size += writer.add_entry_part(part)?;
            }
            writer.finalize()?;
            if part_num == 1 {
                fs::rename(part_name(&archive, 1).unwrap(), &archive)?;
            }
        } else {
            let file = File::create(&archive)?;
            let mut writer = Archive::write_solid_header(file, cli_option)?;
            for entry in rx.into_iter() {
                writer.add_entry(entry?)?;
                progress_bar.let_ref(|pb| pb.inc(1));
            }
            writer.finalize()?;
        }
    } else {
        // if splitting is enabled
        if let Some(max_file_size) = max_file_size {
            let mut part_num = 1;
            let file = File::create(part_name(&archive, part_num).unwrap())?;
            let mut writer = Archive::write_header(file)?;

            // NOTE: max_file_size - (PNA_HEADER + AHED + ANXT + AEND)
            let max_file_size = max_file_size - (PNA_HEADER.len() + MIN_CHUNK_BYTES_SIZE * 3 + 8);
            let mut written_entry_size = 0;
            for entry in rx.into_iter() {
                let parts = split_to_parts(
                    EntryPart::from(entry?),
                    max_file_size - written_entry_size,
                    max_file_size,
                );
                for part in parts {
                    if written_entry_size + part.bytes_len() > max_file_size {
                        part_num += 1;
                        let part_n_name = part_name(&archive, part_num).unwrap();
                        if verbosity == Verbosity::Verbose {
                            eprintln!("Split: {} to {}", archive.display(), part_n_name.display());
                        }
                        let file = File::create(&part_n_name)?;
                        writer = writer.split_to_next_archive(file)?;
                        written_entry_size = 0;
                    }
                    written_entry_size += writer.add_entry_part(part)?;
                }
                progress_bar.let_ref(|pb| pb.inc(1));
            }
            writer.finalize()?;
            if part_num == 1 {
                fs::rename(part_name(&archive, 1).unwrap(), &archive)?;
            }
        } else {
            let file = File::create(&archive)?;
            let mut writer = Archive::write_header(file)?;
            for entry in rx.into_iter() {
                writer.add_entry(entry?)?;
                progress_bar.let_ref(|pb| pb.inc(1));
            }
            writer.finalize()?;
        }
    }

    progress_bar.let_ref(|pb| pb.finish_and_clear());

    if verbosity != Verbosity::Quite {
        eprintln!(
            "Successfully created an archive in {}",
            HumanDuration(start.elapsed())
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_archive() {
        let args = CreateCommand::parse_from(["create", "c.pna"]);
        assert!(!args.compression.store);

        let args = CreateCommand::parse_from(["create", "c.pna", "--store"]);
        assert!(args.compression.store);
    }

    #[test]
    fn deflate_level() {
        let args = CreateCommand::parse_from(["create", "c.pna"]);
        assert_eq!(args.compression.deflate, None);

        let args = CreateCommand::parse_from(["create", "c.pna", "--deflate"]);
        assert_eq!(args.compression.deflate, Some(None));

        let args = CreateCommand::parse_from(["create", "c.pna", "--deflate", "5"]);
        assert_eq!(args.compression.deflate, Some(Some(5u8)));
    }

    #[test]
    fn zstd_level() {
        let args = CreateCommand::parse_from(["create", "c.pna"]);
        assert_eq!(args.compression.zstd, None);

        let args = CreateCommand::parse_from(["create", "c.pna", "--zstd"]);
        assert_eq!(args.compression.zstd, Some(None));

        let args = CreateCommand::parse_from(["create", "c.pna", "--zstd", "5"]);
        assert_eq!(args.compression.zstd, Some(Some(5u8)));
    }

    #[test]
    fn lzma_level() {
        let args = CreateCommand::parse_from(["create", "c.pna"]);
        assert_eq!(args.compression.xz, None);

        let args = CreateCommand::parse_from(["create", "c.pna", "--xz"]);
        assert_eq!(args.compression.xz, Some(None));

        let args = CreateCommand::parse_from(["create", "c.pna", "--xz", "5"]);
        assert_eq!(args.compression.xz, Some(Some(5u8)));
    }

    #[test]
    fn human_readable_byte_size() {
        let args = CreateCommand::parse_from(["create", "c.pna", "--split", "10KiB"]);
        assert_eq!(args.split, Some(Some(ByteSize::kib(10))))
    }
}
