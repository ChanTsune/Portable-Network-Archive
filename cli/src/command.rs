mod acl;
pub mod append;
pub(crate) mod bsdtar;
pub mod bugreport;
mod chmod;
mod chown;
mod chunk;
pub(super) mod compat;
pub mod complete;
pub(crate) mod concat;
pub(crate) mod core;
pub mod create;
pub mod delete;
pub mod diff;
pub(super) mod experimental;
pub mod extract;
pub mod list;
pub mod migrate;
pub mod sort;
pub mod split;
pub(crate) mod strip;
pub mod update;
pub(crate) mod verify;
pub mod xattr;

use crate::cli::{Cli, Commands, GlobalContext, PasswordArgs};
use std::{fmt, fs, io};

/// Error that maps to a specific process exit code in `main`.
#[derive(Debug)]
pub struct ExitCodeError {
    code: u8,
    source: Option<anyhow::Error>,
}

impl ExitCodeError {
    /// Exit with `code` without printing anything.
    pub(crate) fn silent(code: u8) -> Self {
        Self { code, source: None }
    }

    /// Exit with `code` after `main` prints `source`.
    pub(crate) fn with_source(code: u8, source: anyhow::Error) -> Self {
        Self {
            code,
            source: Some(source),
        }
    }

    /// Process exit code to terminate with.
    pub fn code(&self) -> u8 {
        self.code
    }

    /// Consumes self and returns the wrapped error to print, if any.
    pub fn into_source(self) -> Option<anyhow::Error> {
        self.source
    }
}

impl fmt::Display for ExitCodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Some(err) => fmt::Display::fmt(err, f),
            None => write!(f, "process exited with code {}", self.code),
        }
    }
}

impl std::error::Error for ExitCodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|err| -> &(dyn std::error::Error + 'static) { err.as_ref() })
    }
}

fn ask_password(args: PasswordArgs) -> io::Result<Option<Vec<u8>>> {
    if let Some(path) = args.password_file_raw {
        return Ok(Some(fs::read(path)?));
    }
    if let Some(path) = args.password_file {
        let password = fs::read(path)?;
        if password_bytes_need_raw_mode(&password) {
            log::warn!(
                "password file contains a newline or is not valid UTF-8; --password-file will change to use only the first non-empty UTF-8 line in a future release. If the full file content is your password, use --password-file-raw instead."
            );
        }
        return Ok(Some(password));
    }
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

/// Returns whether `password` will be read differently (or rejected) once
/// `--password-file` switches to first-non-empty-UTF-8-line semantics.
#[inline]
fn password_bytes_need_raw_mode(password: &[u8]) -> bool {
    match std::str::from_utf8(password) {
        Ok(text) => text.contains('\n') || text.contains('\r'),
        Err(_) => true,
    }
}

pub(crate) trait Command {
    fn execute(self, ctx: &GlobalContext) -> anyhow::Result<()>;
}

impl Cli {
    #[inline]
    pub fn execute(self) -> anyhow::Result<()> {
        let ctx = &GlobalContext::new(self.global);
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
            Commands::Migrate(cmd) => cmd.execute(ctx),
            Commands::Xattr(cmd) => cmd.execute(ctx),
            Commands::Complete(cmd) => cmd.execute(ctx),
            Commands::BugReport(cmd) => cmd.execute(ctx),
            Commands::Compat(cmd) => cmd.execute(ctx),
            Commands::Experimental(cmd) => cmd.execute(ctx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::password_bytes_need_raw_mode;

    #[test]
    fn detects_lf_and_cr_as_newline() {
        assert!(!password_bytes_need_raw_mode(b"secret"));
        assert!(password_bytes_need_raw_mode(b"secret\n"));
        assert!(password_bytes_need_raw_mode(b"secret\r\n"));
        assert!(password_bytes_need_raw_mode(b"secret\r"));
        assert!(password_bytes_need_raw_mode(b"line1\nline2"));
    }

    #[test]
    fn detects_invalid_utf8() {
        assert!(password_bytes_need_raw_mode(&[0xff, 0xfe, 0xfd]));
        assert!(!password_bytes_need_raw_mode("secret".as_bytes()));
    }
}
