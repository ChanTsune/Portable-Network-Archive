use clap::Parser;
use portable_network_archive::cli;

#[hooq::hooq(anyhow)]
fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    cli.init_logger()?;
    cli.execute()
}
