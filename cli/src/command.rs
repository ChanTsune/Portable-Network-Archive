pub mod append;
mod commons;
pub mod create;
#[cfg(feature = "experimental")]
pub(super) mod experimental;
pub mod extract;
pub mod list;

#[cfg(feature = "experimental")]
use self::experimental::ExperimentalCommands;
use crate::cli::{CipherAlgorithmArgs, Cli, Commands, PasswordArgs, Verbosity};
use std::io;

pub fn entry(cli: Cli) -> io::Result<()> {
    match cli.commands {
        Commands::Create(args) => args.execute(cli.verbosity.verbosity()),
        Commands::Append(args) => args.execute(cli.verbosity.verbosity()),
        Commands::Extract(args) => args.execute(cli.verbosity.verbosity()),
        Commands::List(args) => args.execute(cli.verbosity.verbosity()),
        #[cfg(feature = "experimental")]
        Commands::Experimental(cmd) => match cmd.command {
            ExperimentalCommands::Split(cmd) => cmd.execute(cli.verbosity.verbosity()),
        },
    }
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

trait Command {
    fn execute(self, verbosity: Verbosity) -> io::Result<()>;
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
