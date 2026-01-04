mod acl;
pub mod append;
pub mod bugreport;
mod chmod;
mod chown;
mod chunk;
pub mod complete;
pub(crate) mod concat;
mod core;
pub mod create;
pub mod delete;
pub mod diff;
pub(super) mod experimental;
pub mod extract;
mod fflag;
pub mod list;
mod migrate;
pub mod sort;
pub mod split;
pub(crate) mod stdio;
pub(crate) mod strip;
pub mod update;
pub mod xattr;

use crate::cli::{CipherAlgorithmArgs, Cli, Commands, GlobalArgs, PasswordArgs};
use std::{fs, io};

fn ask_password(args: PasswordArgs) -> io::Result<Option<Vec<u8>>> {
    if let Some(path) = args.password_file {
        return Ok(Some(fs::read(path)?));
    };
    Ok(match args.password {
        Some(Some(password)) => {
            log::warn!("Using a password on the command line interface can be insecure.");
            Some(password.into_bytes())
        }
        Some(None) => Some(
            gix_prompt::securely("Enter password: ")
                .map_err(io::Error::other)?
                .into_bytes(),
        ),
        None => None,
    })
}

fn check_password(password: &Option<Vec<u8>>, cipher_args: &CipherAlgorithmArgs) {
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

pub(crate) trait Command {
    fn execute(self, ctx: &GlobalArgs) -> anyhow::Result<()>;
}

impl Cli {
    #[inline]
    pub fn execute(self) -> anyhow::Result<()> {
        let ctx = &self.global;
        match self.commands {
            Commands::Create(cmd) => cmd.execute(ctx),
            Commands::Append(cmd) => cmd.execute(ctx),
            Commands::Extract(cmd) => cmd.execute(ctx),
            Commands::List(cmd) => cmd.execute(ctx),
            Commands::Delete(cmd) => cmd.execute(ctx),
            Commands::Split(cmd) => cmd.execute(ctx),
            Commands::Concat(cmd) => cmd.execute(ctx),
            Commands::Strip(cmd) => cmd.execute(ctx),
            Commands::Sort(cmd) => cmd.execute(ctx),
            Commands::Xattr(cmd) => cmd.execute(ctx),
            Commands::Complete(cmd) => cmd.execute(ctx),
            Commands::BugReport(cmd) => cmd.execute(ctx),
            Commands::Experimental(cmd) => cmd.execute(ctx),
        }
    }
}
