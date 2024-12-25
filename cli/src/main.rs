use clap::Parser;
use portable_network_archive::{cli, command::Command};
use std::io;

fn main() -> io::Result<()> {
    let cli = cli::Cli::parse();
    cli.init_logger()?;
    cli.execute()
}
