use pna::{clap::Parser, cli, command};
use std::io;

fn main() -> io::Result<()> {
    command::entry(cli::Cli::parse())
}
