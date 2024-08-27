pub mod append;
mod chmod;
mod chown;
mod commons;
pub mod complete;
pub(crate) mod concat;
pub mod create;
mod delete;
pub(super) mod experimental;
pub mod extract;
pub mod list;
pub mod split;
pub(crate) mod stdio;
pub(crate) mod strip;
pub mod update;
mod xattr;

use crate::cli::{CipherAlgorithmArgs, Cli, Commands, PasswordArgs, Verbosity};
use std::{fs, io};

pub fn entry(cli: Cli) -> io::Result<()> {
    let verbosity = match cli.verbosity.log_level() {
        None => Verbosity::Quite,
        Some(log::Level::Error | log::Level::Warn) => Verbosity::Normal,
        Some(log::Level::Trace | log::Level::Debug | log::Level::Info) => Verbosity::Verbose,
    };
    match cli.commands {
        Commands::Create(cmd) => cmd.execute(verbosity),
        Commands::Append(cmd) => cmd.execute(verbosity),
        Commands::Extract(cmd) => cmd.execute(verbosity),
        Commands::List(cmd) => cmd.execute(verbosity),
        Commands::Split(cmd) => cmd.execute(verbosity),
        Commands::Concat(cmd) => cmd.execute(verbosity),
        Commands::Strip(cmd) => cmd.execute(verbosity),
        Commands::Complete(cmd) => cmd.execute(verbosity),
        Commands::Experimental(cmd) => cmd.execute(verbosity),
    }
}

fn ask_password(args: PasswordArgs) -> io::Result<Option<String>> {
    if let Some(path) = args.password_file {
        return Ok(Some(fs::read_to_string(path)?));
    };
    Ok(match args.password {
        Some(password @ Some(_)) => {
            eprintln!("warning: Using a password on the command line interface can be insecure.");
            password
        }
        Some(None) => Some(rpassword::prompt_password("Enter password: ")?),
        None => None,
    })
}

fn check_password(password: &Option<String>, cipher_args: &CipherAlgorithmArgs) {
    if password.is_some() {
        return;
    }
    if cipher_args.aes.is_some() {
        eprintln!("warning: Using `--aes` option but, `--password` was not provided. It will not encrypt.");
    } else if cipher_args.camellia.is_some() {
        eprintln!("warning: Using `--camellia` option but, `--password` was not provided. It will not encrypt.");
    }
}

trait Command {
    fn execute(self, verbosity: Verbosity) -> io::Result<()>;
}
