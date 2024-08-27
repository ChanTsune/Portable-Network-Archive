mod chunk;
mod cli;
mod command;
mod utils;

use clap::Parser;
use std::io;

fn main() -> io::Result<()> {
    let args = cli::Cli::parse();
    init_logger(args.verbosity.log_level_filter())?;
    command::entry(args)
}

fn init_logger(level: log::LevelFilter) -> io::Result<()> {
    let base = fern::Dispatch::new();
    let stderr = fern::Dispatch::new().level(level).chain(io::stderr());
    base.chain(stderr).apply().map_err(io::Error::other)
}
