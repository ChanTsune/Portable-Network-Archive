pub mod append;
mod commons;
pub mod complete;
mod concat;
pub mod create;
mod delete;
pub(super) mod experimental;
pub mod extract;
pub mod list;
pub mod split;
pub(crate) mod stdio;
mod strip;
pub mod update;

use crate::cli::{CipherAlgorithmArgs, Cli, Commands, PasswordArgs, Verbosity};
use std::{fs, io};

pub fn entry(cli: Cli) -> io::Result<()> {
    match cli.commands {
        Commands::Create(cmd) => cmd.execute(cli.verbosity.verbosity()),
        Commands::Append(cmd) => cmd.execute(cli.verbosity.verbosity()),
        Commands::Extract(cmd) => cmd.execute(cli.verbosity.verbosity()),
        Commands::List(cmd) => cmd.execute(cli.verbosity.verbosity()),
        Commands::Split(cmd) => cmd.execute(cli.verbosity.verbosity()),
        Commands::Complete(cmd) => cmd.execute(cli.verbosity.verbosity()),
        Commands::Experimental(cmd) => cmd.execute(cli.verbosity.verbosity()),
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
