pub mod append;
mod commons;
pub mod create;
pub mod extract;
pub mod list;

use crate::cli::{CipherAlgorithmArgs, Cli, Commands, PasswordArgs};
use std::io;

pub fn entry(cli: Cli) -> io::Result<()> {
    match cli.commands {
        Commands::Create(args) => {
            create::create_archive(args, cli.verbosity.verbosity())?;
        }
        Commands::Append(args) => append::append_to_archive(args, cli.verbosity.verbosity())?,
        Commands::Extract(args) => {
            extract::extract_archive(args, cli.verbosity.verbosity())?;
        }
        Commands::List(args) => {
            list::list_archive(args, cli.verbosity.verbosity())?;
        }
    }
    Ok(())
}

fn ask_password(args: PasswordArgs) -> io::Result<Option<String>> {
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

trait Let<T> {
    fn let_ref<U, F: FnOnce(&T) -> U>(&self, f: F);
}

impl<T> Let<T> for Option<T> {
    #[inline]
    fn let_ref<U, F: FnOnce(&T) -> U>(&self, f: F) {
        if let Some(t) = self.as_ref() {
            f(t);
        }
    }
}
