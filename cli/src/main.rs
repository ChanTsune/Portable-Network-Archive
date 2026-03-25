use clap::Parser;
use portable_network_archive::cli;

#[hooq::hooq(anyhow)]
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    let args = cli::expand_bsdtar_old_style_args(args);
    let args = cli::expand_bsdtar_w_option(args);
    let cli = cli::Cli::parse_from(args);
    cli.init_logger()?;
    cli.execute()
}
