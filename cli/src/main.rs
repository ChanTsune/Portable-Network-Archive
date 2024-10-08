mod chunk;
mod cli;
mod command;
mod ext;
mod utils;

use anyhow::{Context, Result};
use clap::Parser;
use command::Command;
use log::Level;
use std::io;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    init_logger(cli.verbosity.log_level_filter())?;
    cli.execute()
}

fn init_logger(level: log::LevelFilter) -> Result<()> {
    let base = fern::Dispatch::new();
    let stderr = fern::Dispatch::new()
        .level(level)
        .format(|out, msg, rec| match rec.level() {
            Level::Error => out.finish(format_args!("error: {}", msg)),
            Level::Warn => out.finish(format_args!("warning: {}", msg)),
            Level::Info | Level::Debug | Level::Trace => out.finish(*msg),
        })
        .chain(io::stderr());
    base.chain(stderr)
        .apply()
        .with_context(|| "failed to initialize logger")
}
