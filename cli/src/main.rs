mod command;

use clap::Parser;
use std::io;

fn main() -> io::Result<()> {
    command::entry(command::Args::parse())
}
