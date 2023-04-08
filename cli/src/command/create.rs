use crate::{
    cli::{CipherAlgorithmArgs, CompressionAlgorithmArgs, CreateArgs, Verbosity},
    command::{ask_password, check_password, Let},
    utils::part_name,
};
use bytesize::ByteSize;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use libpna::{
    ArchiveWriter, Entry, EntryBuilder, EntryPart, Permission, MIN_CHUNK_BYTES_SIZE, PNA_HEADER,
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
            format!("{} is alrady exists", archive.display()),
        ));
    }
    if verbosity != Verbosity::Quite {
        println!("Create an archive: {}", archive.display());
    }
    let mut target_items = vec![];
    for p in args.file.files {
        collect_items(&mut target_items, p.as_ref(), args.recursive)?;
    }

    let progress_bar = if verbosity != Verbosity::Quite {
        Some(
            ProgressBar::new(target_items.len() as u64)
                .with_style(ProgressStyle::default_bar().progress_chars("=> ")),
        )
    } else {
        None
    };

    let item_count = target_items.len();

    let (tx, rx) = std::sync::mpsc::channel();
    for file in target_items {
        let compression = args.compression.clone();
        let cipher = args.cipher.clone();
        let password = password.clone();
        let keep_timestamp = args.keep_timestamp;
        let keep_permission = args.keep_permission;
        let tx = tx.clone();
        pool.spawn_fifo(move || {
            tx.send(write_internal(
                &file,
                compression,
                cipher,
                password,
                keep_timestamp,
                keep_permission,
                verbosity,
            ))
            .unwrap_or_else(|e| panic!("{e}: {}", file.display()));
        });
    }

    drop(tx);

    if let Some(parent) = archive.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(&archive)?;
    let mut writer = ArchiveWriter::write_header(file)?;

    let max_file_size = args
        .split
        .map(|it| it.unwrap_or(ByteSize::gb(1)).0 as usize);
    // if splitting is enabled
    if let Some(max_file_size) = max_file_size {
        // NOTE: max_file_size - (PNA_HEADER + AHED + ANXT + AEND)
        let max_file_size = max_file_size - (PNA_HEADER.len() + MIN_CHUNK_BYTES_SIZE * 3 + 8);
        let mut part_num = 0;
        let mut written_entry_size = 0;
        for (idx, item) in rx.into_iter().enumerate() {
            let entry = item?;
            let is_last = idx + 1 == item_count;
            if written_entry_size + entry.bytes_len() >= max_file_size {
                let mut parts = vec![];
                let mut entry_part = EntryPart::from(entry);
                loop {
                    match entry_part.split(max_file_size - written_entry_size) {
                        (write_part, Some(remaining_part)) => {
                            parts.push(write_part);
                            entry_part = remaining_part;
                        }
                        (write_part, None) => {
                            parts.push(write_part);
                            break;
                        }
                    }
                }
                let part_len = parts.len();
                for (is_last_part, part) in parts
                    .into_iter()
                    .enumerate()
                    .map(|(idx, p)| (idx + 1 == part_len, p))
                {
                    written_entry_size += writer.add_entry_part(part)?;
                    part_num += 1;
                    let part_n_name = part_name(&archive, part_num).unwrap();
                    if verbosity == Verbosity::Verbose {
                        println!("Split: {} to {}", archive.display(), part_n_name.display());
                    }
                    fs::rename(&archive, part_n_name)?;
                    if !(is_last && is_last_part) {
                        let file = File::create(&archive)?;
                        writer = writer.split_to_next_archive(file)?;
                        written_entry_size = 0;
                    }
                }
            } else {
                written_entry_size += writer.add_entry(entry)?;
            }
            progress_bar.let_ref(|pb| pb.inc(1));
        }
    } else {
        for entry in rx.into_iter() {
            writer.add_entry(entry?)?;
            progress_bar.let_ref(|pb| pb.inc(1));
        }
    }
    writer.finalize()?;

    progress_bar.let_ref(|pb| pb.finish_and_clear());

    if verbosity != Verbosity::Quite {
        println!(
            "Successfully created an archive in {}",
            HumanDuration(start.elapsed())
        );
    }
    Ok(())
}

fn collect_items(result: &mut Vec<PathBuf>, path: &Path, recursive: bool) -> io::Result<()> {
    if path.is_dir() {
        if recursive {
            for p in fs::read_dir(path)? {
                collect_items(result, &p?.path(), recursive)?;
            }
        }
    } else if path.is_file() {
        result.push(path.to_path_buf());
    }
    Ok(())
}

fn write_internal(
    path: &Path,
    compression: CompressionAlgorithmArgs,
    cipher: CipherAlgorithmArgs,
    password: Option<String>,
    keep_timestamp: bool,
    keep_permission: bool,
    verbosity: Verbosity,
) -> io::Result<impl Entry> {
    if verbosity == Verbosity::Verbose {
        println!("Adding: {}", path.display());
    }
    if path.is_file() {
        let mut option_builder = libpna::WriteOptionBuilder::default();
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
        let mut entry = EntryBuilder::new_file(path.into(), option_builder.build())?;
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
        entry.write_all(&fs::read(path)?)?;
        return entry.build();
    }
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Currently not a regular file is not supported.",
    ))
}
