use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs, CreateArgs, Verbosity},
    command::{ask_password, check_password, Let},
    utils::part_name,
};
use bytesize::ByteSize;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use libpna::{
    ArchiveWriter, Entry, EntryBuilder, EntryPart, Permission, SolidEntriesBuilder, WriteOption,
    WriteOptionBuilder, MIN_CHUNK_BYTES_SIZE, PNA_HEADER,
};
#[cfg(unix)]
use nix::unistd::{Group, User};
use rayon::ThreadPoolBuilder;
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::{
    fs::{self, metadata, File},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Instant, UNIX_EPOCH},
};

pub(crate) fn create_archive(args: CreateArgs, verbosity: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    check_password(&password, &args.cipher);
    let start = Instant::now();
    let pool = ThreadPoolBuilder::default()
        .build()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let archive = args.file.archive;
    if !args.overwrite && archive.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is already exists", archive.display()),
        ));
    }
    if verbosity != Verbosity::Quite {
        println!("Create an archive: {}", archive.display());
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
        let tx = tx.clone();
        pool.spawn_fifo(move || {
            if verbosity == Verbosity::Verbose {
                println!("Adding: {}", file.display());
            }
            tx.send(create_entry(&file, option, keep_timestamp, keep_permission))
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
        let mut entries_builder = SolidEntriesBuilder::new(cli_option)?;
        for entry in rx.into_iter() {
            entries_builder.add_entry(entry?)?;
            progress_bar.let_ref(|pb| pb.inc(1));
        }
        let entries = entries_builder.build()?;
        if let Some(max_file_size) = max_file_size {
            let mut part_num = 1;
            let file = File::create(part_name(&archive, part_num).unwrap())?;
            let mut writer = ArchiveWriter::write_header(file)?;

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
                        println!("Split: {} to {}", archive.display(), part_n_name.display());
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
            let mut writer = ArchiveWriter::write_header(file)?;
            writer.add_solid_entries(entries)?;
            writer.finalize()?;
        }
    } else {
        // if splitting is enabled
        if let Some(max_file_size) = max_file_size {
            let mut part_num = 1;
            let file = File::create(part_name(&archive, part_num).unwrap())?;
            let mut writer = ArchiveWriter::write_header(file)?;

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
                            println!("Split: {} to {}", archive.display(), part_n_name.display());
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
            let mut writer = ArchiveWriter::write_header(file)?;
            for entry in rx.into_iter() {
                writer.add_entry(entry?)?;
                progress_bar.let_ref(|pb| pb.inc(1));
            }
            writer.finalize()?;
        }
    }

    progress_bar.let_ref(|pb| pb.finish_and_clear());

    if verbosity != Verbosity::Quite {
        println!(
            "Successfully created an archive in {}",
            HumanDuration(start.elapsed())
        );
    }
    Ok(())
}

fn collect_items(files: Vec<PathBuf>, recursive: bool, keep_dir: bool) -> io::Result<Vec<PathBuf>> {
    fn collect_items(
        result: &mut Vec<PathBuf>,
        path: &Path,
        recursive: bool,
        keep_dir: bool,
    ) -> io::Result<()> {
        if path.is_dir() {
            if keep_dir {
                result.push(path.to_path_buf());
            }
            if recursive {
                for p in fs::read_dir(path)? {
                    collect_items(result, &p?.path(), recursive, keep_dir)?;
                }
            }
        } else if path.is_file() {
            result.push(path.to_path_buf());
        }
        Ok(())
    }
    let mut target_items = vec![];
    for p in files {
        collect_items(&mut target_items, p.as_ref(), recursive, keep_dir)?;
    }
    Ok(target_items)
}

fn split_to_parts(mut entry_part: EntryPart, first: usize, max: usize) -> Vec<EntryPart> {
    let mut parts = vec![];
    let mut split_size = first;
    loop {
        match entry_part.split(split_size) {
            (write_part, Some(remaining_part)) => {
                parts.push(write_part);
                entry_part = remaining_part;
                split_size = max;
            }
            (write_part, None) => {
                parts.push(write_part);
                break;
            }
        }
    }
    parts
}

fn create_entry(
    path: &Path,
    option: WriteOption,
    keep_timestamp: bool,
    keep_permission: bool,
) -> io::Result<impl Entry> {
    if path.is_file() {
        let mut entry = EntryBuilder::new_file(path.into(), option)?;
        entry.write_all(&fs::read(path)?)?;
        return apply_metadata(entry, path, keep_timestamp, keep_permission)?.build();
    } else if path.is_dir() {
        let entry = EntryBuilder::new_dir(path.into());
        return apply_metadata(entry, path, keep_timestamp, keep_permission)?.build();
    }
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Currently not a regular file is not supported.",
    ))
}

fn entry_option(
    compression: CompressionAlgorithmArgs,
    cipher: CipherAlgorithmArgs,
    password: Option<String>,
) -> WriteOption {
    let mut option_builder = WriteOptionBuilder::default();
    let (algorithm, level) = compression.algorithm();
    option_builder.compression(algorithm);
    if let Some(level) = level {
        option_builder.compression_level(level);
    }
    option_builder
        .encryption(if password.is_some() {
            cipher.algorithm()
        } else {
            libpna::Encryption::No
        })
        .cipher_mode(cipher.mode())
        .password(password);
    option_builder.build()
}

fn apply_metadata(
    mut entry: EntryBuilder,
    path: &Path,
    keep_timestamp: bool,
    keep_permission: bool,
) -> io::Result<EntryBuilder> {
    if keep_timestamp || keep_permission {
        let meta = metadata(path)?;
        if keep_timestamp {
            if let Ok(c) = meta.created() {
                if let Ok(created_since_unix_epoch) = c.duration_since(UNIX_EPOCH) {
                    entry.created(created_since_unix_epoch);
                }
            }
            if let Ok(m) = meta.modified() {
                if let Ok(modified_since_unix_epoch) = m.duration_since(UNIX_EPOCH) {
                    entry.modified(modified_since_unix_epoch);
                }
            }
        }
        #[cfg(unix)]
        if keep_permission {
            let mode = meta.permissions().mode() as u16;
            let uid = meta.uid();
            let gid = meta.gid();
            let user = User::from_uid(uid.into())?.unwrap();
            let group = Group::from_gid(gid.into())?.unwrap();
            entry.permission(Permission::new(
                uid.into(),
                user.name,
                gid.into(),
                group.name,
                mode,
            ));
        }
    }
    Ok(entry)
}
