use clap::Parser;
use portable_network_archive::cli;
use std::{io, process::ExitCode};

#[hooq::hooq(anyhow)]
fn run() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    let args = cli::expand_bsdtar_old_style_args(args);
    let args = cli::expand_bsdtar_w_option(args);
    let cli = cli::Cli::parse_from(args);
    cli.init_logger()?;
    cli.execute()
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) if is_broken_pipe(&err) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {err:?}");
            ExitCode::FAILURE
        }
    }
}

fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.chain()
        .filter_map(|cause| cause.downcast_ref::<io::Error>())
        .any(|io_err| io_err.kind() == io::ErrorKind::BrokenPipe)
}
