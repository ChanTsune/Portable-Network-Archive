mod cli;
mod command;

use clap::Parser;
use std::io;

fn main() -> io::Result<()> {
    command::entry(cli::Cli::parse())
}
