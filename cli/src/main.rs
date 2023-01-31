mod create;
mod extract;

use clap::{value_parser, ArgGroup, Parser};
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
            .args(["create", "append", "extract"]),
    )
 )]
struct Args {
    #[arg(short, long, value_name = "ARCHIVE", help = "Create archive")]
    create: Option<PathBuf>,
    #[arg(short, long, value_name = "ARCHIVE", help = "Append files to archive")]
    append: Option<PathBuf>,
    #[arg(short = 'x', long, value_name = "ARCHIVE", help = "Extract archive")]
    extract: Option<PathBuf>,
    #[command(flatten)]
    options: Options,
    #[arg()]
    files: Vec<PathBuf>,
}

#[derive(Parser)]
struct Options {
    #[arg(short, long, help = "Add the directory to the archive recursively")]
    recursive: bool,
    #[arg(long, help = "Overwrite file")]
    overwrite: bool,
    #[arg(long, default_value = "true", help = "Use zstd for compression")]
    zstd: bool,
    #[arg(
        long,
        value_name = "level",
        value_parser = value_parser!(u8).range(1..=21),
        help = "Zstd compression level [level: 1-21]"
    )]
    zstd_level: Option<u8>,
    #[arg(long, help = "Use xz for compression")]
    lzma: bool,
    #[arg(
        long,
        value_name = "level",
        value_parser = value_parser!(u8).range(0..=9),
        help = "Xz compression level [level: 0-9]"
    )]
    lzma_level: Option<u8>,
    #[arg(long, help = "Make some output more verbose")]
    verbose: bool,
    #[arg(long, help = "Make some output more quiet")]
    quiet: bool,
}

fn main() -> io::Result<()> {
    entry(Args::parse())
}

fn entry(args: Args) -> io::Result<()> {
    if let Some(create) = args.create {
        create::create_archive(create, &args.files, args.options)?;
    } else if let Some(append) = args.append {
        println!("Append archive {}", append.display());
    } else if let Some(extract) = args.extract {
        extract::extract_archive(extract, &args.files, args.options)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{entry, Args};
    use clap::Parser;

    #[test]
    fn create_archive() {
        entry(Args::parse_from([
            "pna",
            "--overwrite",
            "--quiet",
            "-c",
            "c.pna",
        ]))
        .unwrap();
    }

    #[test]
    fn zstd_level() {
        let args = Args::parse_from(["pna", "-c", "c.pna"]);
        assert_eq!(args.options.zstd, true);
        let args = Args::parse_from(["pna", "-c", "c.pna", "--zstd"]);
        assert_eq!(args.options.zstd, true);
        let args = Args::parse_from(["pna", "-c", "c.pna", "--zstd-level", "5"]);
        assert_eq!(args.options.zstd_level, Some(5u8))
    }

    #[test]
    fn lzma_level() {
        let args = Args::parse_from(["pna", "-c", "c.pna"]);
        assert_eq!(args.options.lzma, false);
        let args = Args::parse_from(["pna", "-c", "c.pna", "--lzma"]);
        assert_eq!(args.options.lzma, true);
        let args = Args::parse_from(["pna", "-c", "c.pna", "--lzma-level", "5"]);
        assert_eq!(args.options.lzma_level, Some(5u8))
    }
}
