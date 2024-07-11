use crate::{
    cli::{PasswordArgs, Verbosity},
    command::{
        ask_password,
        commons::{run_manipulate_entry_by_path, run_process_archive_path},
        Command,
    },
    utils::{GlobPatterns, PathPartExt},
};
use base64::Engine;
use clap::{Parser, ValueHint};
use indexmap::IndexMap;
use std::{
    fmt::{Display, Formatter},
    io,
    path::PathBuf,
    str::FromStr,
};

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
#[command(args_conflicts_with_subcommands = true, arg_required_else_help = true)]
pub(crate) struct XattrCommand {
    #[command(subcommand)]
    command: XattrCommands,
}

impl Command for XattrCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        match self.command {
            XattrCommands::Get(cmd) => cmd.execute(verbosity),
            XattrCommands::Set(cmd) => cmd.execute(verbosity),
        }
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) enum XattrCommands {
    #[command(about = "Get extended attributes of entries")]
    Get(GetXattrCommand),
    #[command(about = "Set extended attributes of entries")]
    Set(SetXattrCommand),
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct GetXattrCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[arg(short, long, help = "Filter by name of extended attribute")]
    name: Option<String>,
    #[arg(short, long, help = "Value encoding")]
    encoding: Option<Encoding>,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for GetXattrCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        archive_get_xattr(self, verbosity)
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct SetXattrCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[arg(short, long, help = "Name of extended attribute")]
    name: Option<String>,
    #[arg(short, long, help = "Value of extended attribute")]
    value: Option<String>,
    #[arg(short = 'x', long, help = "Remove extended attribute")]
    remove: Option<String>,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for SetXattrCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        archive_set_xattr(self, verbosity)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
enum Encoding {
    #[default]
    Text,
    Hex,
    Base64,
}

impl Display for Encoding {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Encoding::Text => "text",
            Encoding::Hex => "hex",
            Encoding::Base64 => "base64",
        })
    }
}

impl FromStr for Encoding {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(Self::Text),
            "hex" => Ok(Self::Hex),
            "base64" => Ok(Self::Base64),
            _ => Err("only allowed `text`, `hex` or `base64`".into()),
        }
    }
}

fn archive_get_xattr(args: GetXattrCommand, _: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    if args.files.is_empty() {
        return Ok(());
    }
    let globs = GlobPatterns::new(args.files)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let encoding = args.encoding.unwrap_or_default();

    run_process_archive_path(
        &args.archive,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            let name = entry.header().path().as_ref();
            if globs.matches_any(name) {
                println!("{}", name);
                for attr in entry.xattrs().iter().filter(|a| {
                    args.name.is_none() || args.name.as_deref().is_some_and(|it| it == a.name())
                }) {
                    match encoding {
                        Encoding::Text => {
                            println!("{}: {}", attr.name(), DisplayText(attr.value()));
                        }
                        Encoding::Hex => {
                            println!("{}: {}", attr.name(), DisplayHex(attr.value()));
                        }
                        Encoding::Base64 => {
                            println!("{}: {}", attr.name(), DisplayBase64(attr.value()));
                        }
                    }
                }
            }
            Ok(())
        },
    )?;
    Ok(())
}

fn archive_set_xattr(args: SetXattrCommand, _: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    if args.files.is_empty() {
        return Ok(());
    }
    let globs = GlobPatterns::new(args.files)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    run_manipulate_entry_by_path(
        args.archive.remove_part().unwrap(),
        &args.archive,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            let name = entry.header().path().as_ref();
            if globs.matches_any(name) {
                let mut xattrs = entry
                    .xattrs()
                    .iter()
                    .map(|it| (it.name(), it.value()))
                    .collect::<IndexMap<_, _>>();
                if let Some(name) = args.name.as_deref() {
                    let map_entry = xattrs.entry(name);
                    map_entry.or_insert(args.value.as_deref().unwrap_or_default().as_bytes());
                }
                if let Some(name) = args.name.as_deref() {
                    xattrs.shift_remove_entry(name);
                }
                let xattrs = xattrs
                    .into_iter()
                    .map(|(key, value)| pna::ExtendedAttribute::new(key.into(), value.into()))
                    .collect::<Vec<_>>();
                Ok(entry.with_xattrs(&xattrs))
            } else {
                Ok(entry)
            }
        },
    )
}

struct DisplayText<'a>(&'a [u8]);

impl<'a> Display for DisplayText<'a> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match std::str::from_utf8(self.0) {
            Ok(s) => f.write_str(s),
            Err(e) => write!(f, "{}", e),
        }
    }
}

struct DisplayHex<'a>(&'a [u8]);

impl<'a> Display for DisplayHex<'a> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("0x")?;
        for i in self.0 {
            write!(f, "{:x}", i)?;
        }
        Ok(())
    }
}

struct DisplayBase64<'a>(&'a [u8]);

impl<'a> Display for DisplayBase64<'a> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("0s")?;
        f.write_str(&base64::engine::general_purpose::STANDARD.encode(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_text() {
        let v = DisplayText(b"abc");
        assert_eq!(format!("{}", v), "abc");
    }

    #[test]
    fn encode_hex() {
        let v = DisplayHex(b"abc");
        assert_eq!(format!("{}", v), "0x616263");
    }

    #[test]
    fn encode_base64() {
        let v = DisplayBase64(b"abc");
        assert_eq!(format!("{}", v), "0sYWJj");
    }
}
