use clap::Parser;
use portable_network_archive::{cli, command::Command};

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    cli.init_logger()?;
    #[cfg(target_family = "wasm")]
    rayon::ThreadPoolBuilder::new()
        .use_current_thread()
        .build_global()?;
    cli.execute()
}
