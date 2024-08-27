mod chunk;
mod cli;
mod command;
mod utils;

use clap::Parser;
use std::io;

fn main() -> io::Result<()> {
    command::entry(cli::Cli::parse())
}
