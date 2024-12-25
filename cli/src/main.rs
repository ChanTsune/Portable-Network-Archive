mod chunk;
mod cli;
mod command;
mod ext;
mod utils;

use clap::Parser;
use command::Command;
use std::io;

fn main() -> io::Result<()> {
    let cli = cli::Cli::parse();
    cli.init_logger()?;
    cli.execute()
}
