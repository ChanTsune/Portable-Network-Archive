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
        Commands::Append(args) => todo!("Append archive {}", args.file.archive.display()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn store_archive() {
        let args = Cli::parse_from(["pna", "c", "c.pna"]);
        if let Commands::Create(args) = args.commands {
            assert!(!args.compression.store);
        } else {
            unreachable!()
        }
        let args = Cli::parse_from(["pna", "c", "c.pna", "--store"]);
        if let Commands::Create(args) = args.commands {
            assert!(args.compression.store);
        } else {
            unreachable!()
        }
    }

    #[test]
    fn deflate_level() {
        let args = Cli::parse_from(["pna", "c", "c.pna"]);
        if let Commands::Create(args) = args.commands {
            assert_eq!(args.compression.deflate, None);
        } else {
            unreachable!()
        }

        let args = Cli::parse_from(["pna", "c", "c.pna", "--deflate"]);
        if let Commands::Create(args) = args.commands {
            assert_eq!(args.compression.deflate, Some(None));
        } else {
            unreachable!()
        }
        let args = Cli::parse_from(["pna", "c", "c.pna", "--deflate", "5"]);
        if let Commands::Create(args) = args.commands {
            assert_eq!(args.compression.deflate, Some(Some(5u8)));
        } else {
            unreachable!()
        }
    }

    #[test]
    fn zstd_level() {
        let args = Cli::parse_from(["pna", "c", "c.pna"]);
        if let Commands::Create(args) = args.commands {
            assert_eq!(args.compression.zstd, None);
        } else {
            unreachable!()
        }
        let args = Cli::parse_from(["pna", "c", "c.pna", "--zstd"]);
        if let Commands::Create(args) = args.commands {
            assert_eq!(args.compression.zstd, Some(None));
        } else {
            unreachable!()
        }
        let args = Cli::parse_from(["pna", "c", "c.pna", "--zstd", "5"]);
        if let Commands::Create(args) = args.commands {
            assert_eq!(args.compression.zstd, Some(Some(5u8)));
        } else {
            unreachable!()
        }
    }

    #[test]
    fn lzma_level() {
        let args = Cli::parse_from(["pna", "c", "c.pna"]);
        if let Commands::Create(args) = args.commands {
            assert_eq!(args.compression.xz, None);
        } else {
            unreachable!()
        }
        let args = Cli::parse_from(["pna", "c", "c.pna", "--xz"]);
        if let Commands::Create(args) = args.commands {
            assert_eq!(args.compression.xz, Some(None));
        } else {
            unreachable!()
        }
        let args = Cli::parse_from(["pna", "c", "c.pna", "--xz", "5"]);
        if let Commands::Create(args) = args.commands {
            assert_eq!(args.compression.xz, Some(Some(5u8)));
        } else {
            unreachable!()
        }
    }
}
