use clap::Parser;
use portable_network_archive::cli;
use portable_network_archive::command::ExitCodeError;
use std::io;

#[hooq::hooq(anyhow)]
fn run() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    let args = cli::expand_bsdtar_old_style_args(args);
    let args = cli::expand_bsdtar_w_option(args);
    let cli = cli::Cli::parse_from(args);
    cli.init_logger()?;
    cli.execute()
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) if is_broken_pipe(&err) => std::process::ExitCode::SUCCESS,
        Err(err) => match err.downcast::<ExitCodeError>() {
            Ok(exit_err) => {
                let code = exit_err.code();
                if let Some(source) = exit_err.into_source() {
                    eprintln!("Error: {source:?}");
                }
                std::process::ExitCode::from(code)
            }
            Err(err) => {
                eprintln!("Error: {err:?}");
                std::process::ExitCode::FAILURE
            }
        },
    }
}

fn is_broken_pipe(err: &anyhow::Error) -> bool {
    err.chain()
        .filter_map(|cause| cause.downcast_ref::<io::Error>())
        .any(|io_err| io_err.kind() == io::ErrorKind::BrokenPipe)
}
