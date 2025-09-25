mod acl;
pub mod append;
pub mod bugreport;
mod chmod;
mod chown;
mod chunk;
mod commons;
pub mod complete;
pub(crate) mod concat;
pub mod create;
mod delete;
pub(super) mod experimental;
pub mod extract;
pub mod list;
mod migrate;
mod sort;
pub mod split;
pub(crate) mod stdio;
pub(crate) mod strip;
pub mod update;
pub mod xattr;

use crate::cli::{CipherAlgorithmArgs, Cli, Commands, PasswordArgs};
use std::{fs, io};

fn ask_password(args: PasswordArgs) -> io::Result<Option<String>> {
    if let Some(path) = args.password_file {
        let mut password = fs::read_to_string(path)?;
        // Trim common line terminators to avoid accidental newline in passwords read from files
        while password.ends_with('\n') || password.ends_with('\r') {
            password.pop();
        }
        return Ok(Some(password));
    };
    Ok(match args.password {
        Some(password @ Some(_)) => {
            log::warn!("Using a password on the command line interface can be insecure.");
            password
        }
        Some(None) => Some(gix_prompt::securely("Enter password: ").map_err(io::Error::other)?),
        None => None,
    })
}

fn check_password(password: &Option<String>, cipher_args: &CipherAlgorithmArgs) {
    if password.is_some() {
        return;
    }
    if cipher_args.aes.is_some() {
        log::warn!("Using `--aes` option but, `--password` was not provided. It will not encrypt.");
    } else if cipher_args.camellia.is_some() {
        log::warn!(
            "Using `--camellia` option but, `--password` was not provided. It will not encrypt."
        );
    }
}

pub trait Command {
    fn execute(self) -> anyhow::Result<()>;
}

impl Command for Cli {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        match self.commands {
            Commands::Create(cmd) => cmd.execute(),
            Commands::Append(cmd) => cmd.execute(),
            Commands::Extract(cmd) => cmd.execute(),
            Commands::List(cmd) => cmd.execute(),
            Commands::Split(cmd) => cmd.execute(),
            Commands::Concat(cmd) => cmd.execute(),
            Commands::Strip(cmd) => cmd.execute(),
            Commands::Xattr(cmd) => cmd.execute(),
            Commands::Complete(cmd) => cmd.execute(),
            Commands::BugReport(cmd) => cmd.execute(),
            Commands::Experimental(cmd) => cmd.execute(),
        }
    }
}
