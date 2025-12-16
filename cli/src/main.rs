use clap::Parser;
use portable_network_archive::cli;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    cli.init_logger()?;
    cli.execute()
}
