use crate::{
    cli::Verbosity,
    command::{commons::split_to_parts, Command},
    utils::part_name,
};
use bytesize::ByteSize;
use clap::{Parser, ValueHint};
use pna::{Archive, EntryPart, MIN_CHUNK_BYTES_SIZE, PNA_HEADER};
use std::{fs, fs::File, io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct SplitCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    pub(crate) archive: PathBuf,
    #[arg(long, value_hint = ValueHint::DirPath)]
    pub(crate) out_dir: Option<PathBuf>,
    #[arg(long, help = "Overwrite file")]
    pub(crate) overwrite: bool,
    #[arg(long, help = "Maximum size of split archive")]
    pub(crate) max_size: Option<ByteSize>,
}

impl Command for SplitCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        split_archive(self, verbosity)
    }
}

fn split_archive(args: SplitCommand, verbosity: Verbosity) -> io::Result<()> {
    let read_file = File::open(&args.archive)?;
    let base_out_file_name = if let Some(out_dir) = args.out_dir {
        fs::create_dir_all(&out_dir)?;
        out_dir.join(args.archive.file_name().unwrap_or_default())
    } else {
        args.archive.clone()
    };
    let mut read_archive = Archive::read_header(read_file)?;

    let mut n = 1;
    let name = part_name(&base_out_file_name, n).unwrap();
    if !args.overwrite && name.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is already exists", name.display()),
        ));
    }
    let file = File::create(name)?;

    let mut writer = Archive::write_header(file)?;

    let max_file_size = args.max_size.unwrap_or_else(|| ByteSize::gb(1)).as_u64() as usize;

    // NOTE: max_file_size - (PNA_HEADER + AHED + ANXT + AEND)
    let max_file_size = max_file_size - (PNA_HEADER.len() + MIN_CHUNK_BYTES_SIZE * 3 + 8);
    let mut written_entry_size = 0;
    for entry in read_archive.raw_entries() {
        let parts = split_to_parts(
            EntryPart::from(entry?),
            max_file_size - written_entry_size,
            max_file_size,
        );
        for part in parts {
            if written_entry_size + part.bytes_len() > max_file_size {
                n += 1;
                let part_n_name = part_name(&base_out_file_name, n).unwrap();
                if verbosity == Verbosity::Verbose {
                    eprintln!(
                        "Split: {} to {}",
                        args.archive.display(),
                        part_n_name.display()
                    );
                }
                if !args.overwrite && part_n_name.exists() {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        format!("{} is already exists", part_n_name.display()),
                    ));
                }
                let file = File::create(&part_n_name)?;
                writer = writer.split_to_next_archive(file)?;
                written_entry_size = 0;
            }
            written_entry_size += writer.add_entry_part(part)?;
        }
    }
    writer.finalize()?;
    Ok(())
}
