//! The main entry point for the PNA command-line interface.
use clap::Parser;
use portable_network_archive::{cli, command::Command};

/// The main entry point of the application.
///
/// This function parses command-line arguments, initializes the logger, and
/// executes the appropriate command.
fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    cli.init_logger()?;
    cli.execute()
}
