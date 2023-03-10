pub mod create;
pub mod extract;
pub mod list;

use clap::{value_parser, ArgGroup, Parser, ValueEnum};
use std::{io, path::PathBuf};

#[derive(Parser)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version,
    about,
    author,
    arg_required_else_help = true,
    group(
        ArgGroup::new("archive")
        .required(true)
        .args(["create", "append", "extract", "list"]),
    )
)]
pub struct Args {
    #[arg(short, long, value_name = "ARCHIVE", help = "Create archive")]
    create: Option<PathBuf>,
    #[arg(short, long, value_name = "ARCHIVE", help = "Append files to archive")]
    append: Option<PathBuf>,
    #[arg(short = 'x', long, value_name = "ARCHIVE", help = "Extract archive")]
    extract: Option<PathBuf>,
    #[arg(short, long, value_name = "ARCHIVE", help = "List archive items")]
    list: Option<PathBuf>,
    #[command(flatten)]
    options: Options,
    #[arg()]
    files: Vec<PathBuf>,
}

#[derive(Parser, Clone)]
pub(crate) struct Options {
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    recursive: bool,
    #[arg(long, help = "Overwrite file")]
    overwrite: bool,
    #[arg(long, help = "Output directory of extracted files")]
    out_dir: Option<PathBuf>,
    #[arg(long, help = "No compression")]
    store: bool,
    #[arg(
        long,
        value_name = "level",
        value_parser = value_parser!(u8).range(1..=9),
        help = "Use deflate for compression [possible level: 1-9]"
    )]
    deflate: Option<Option<u8>>,
    #[arg(
        long,
        value_name = "level",
        value_parser = value_parser!(u8).range(1..=21),
        help = "Use zstd for compression [possible level: 1-21]"
    )]
    zstd: Option<Option<u8>>,
    #[arg(
        long,
        value_name = "level",
        value_parser = value_parser!(u8).range(0..=9),
        help = "Use xz for compression [possible level: 0-9]"
    )]
    lzma: Option<Option<u8>>,
    #[arg(
        long,
        help = "Password of archive. If password is not given it's asked from the tty"
    )]
    password: Option<Option<String>>,
    #[arg(long, value_name = "cipher mode", help = "Use aes for encryption")]
    aes: Option<Option<CipherMode>>,
    #[arg(long, value_name = "cipher mode", help = "use camellia for encryption")]
    camellia: Option<Option<CipherMode>>,
    #[arg(long, help = "Make some output more verbose")]
    verbose: bool,
    #[arg(long, help = "Make some output more quiet")]
    quiet: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum CipherMode {
    Cbc,
    Ctr,
}

impl Default for CipherMode {
    fn default() -> Self {
        Self::Ctr
    }
}

pub fn entry(mut args: Args) -> io::Result<()> {
    match args.options.password {
        Some(Some(_)) => {
            eprintln!("warning: Using a password on the command line interface can be insecure.");
        }
        Some(None) => {
            args.options.password = Some(Some(rpassword::prompt_password("Enter password: ")?));
        }
        None => {
            if args.options.aes.is_some() {
                eprintln!("warning: Using `--aes` option but, `--password` was not provided. It will not encrypt.");
            } else if args.options.camellia.is_some() {
                eprintln!("warning: Using `--camellia` option but, `--password` was not provided. It will not encrypt.");
            }
        }
    }
    if let Some(create) = args.create {
        create::create_archive(create, &args.files, args.options)?;
    } else if let Some(append) = args.append {
        println!("Append archive {}", append.display());
    } else if let Some(extract) = args.extract {
        extract::extract_archive(extract, &args.files, args.options)?;
    } else if let Some(list) = args.list {
        list::list_archive(list, &args.files, args.options)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Args;
    use clap::Parser;

    #[test]
    fn store_archive() {
        let args = Args::parse_from(["pna", "-c", "c.pna"]);
        assert!(!args.options.store);
        let args = Args::parse_from(["pna", "-c", "c.pna", "--store"]);
        assert!(args.options.store);
    }

    #[test]
    fn deflate_level() {
        let args = Args::parse_from(["pna", "-c", "c.pna"]);
        assert_eq!(args.options.deflate, None);
        let args = Args::parse_from(["pna", "-c", "c.pna", "--deflate"]);
        assert_eq!(args.options.deflate, Some(None));
        let args = Args::parse_from(["pna", "-c", "c.pna", "--deflate", "5"]);
        assert_eq!(args.options.deflate, Some(Some(5u8)))
    }

    #[test]
    fn zstd_level() {
        let args = Args::parse_from(["pna", "-c", "c.pna"]);
        assert_eq!(args.options.zstd, None);
        let args = Args::parse_from(["pna", "-c", "c.pna", "--zstd"]);
        assert_eq!(args.options.zstd, Some(None));
        let args = Args::parse_from(["pna", "-c", "c.pna", "--zstd", "5"]);
        assert_eq!(args.options.zstd, Some(Some(5u8)))
    }

    #[test]
    fn lzma_level() {
        let args = Args::parse_from(["pna", "-c", "c.pna"]);
        assert_eq!(args.options.lzma, None);
        let args = Args::parse_from(["pna", "-c", "c.pna", "--lzma"]);
        assert_eq!(args.options.lzma, Some(None));
        let args = Args::parse_from(["pna", "-c", "c.pna", "--lzma", "5"]);
        assert_eq!(args.options.lzma, Some(Some(5u8)))
    }
}
