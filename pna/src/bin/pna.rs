use pna::{clap::Parser, command};
use std::io;

fn main() -> io::Result<()> {
    command::entry(command::Args::parse())
}
