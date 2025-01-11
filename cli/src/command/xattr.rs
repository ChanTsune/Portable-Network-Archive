use crate::{
    cli::{PasswordArgs, SolidEntriesTransformStrategy, SolidEntriesTransformStrategyArgs},
    command::{
        ask_password,
        commons::{
            run_entries, run_transform_entry, TransformStrategyKeepSolid, TransformStrategyUnSolid,
        },
        Command,
    },
    utils::{str::char_chunks, GlobPatterns, PathPartExt},
};
use base64::Engine;
use clap::{Parser, ValueHint};
use indexmap::IndexMap;
use pna::NormalEntry;
use std::{
    fmt::{Display, Formatter, Write},
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
    #[inline]
    fn execute(self) -> io::Result<()> {
        match self.command {
            XattrCommands::Get(cmd) => cmd.execute(),
            XattrCommands::Set(cmd) => cmd.execute(),
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
    #[inline]
    fn execute(self) -> io::Result<()> {
        archive_get_xattr(self)
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
    value: Option<Value>,
    #[arg(short = 'x', long, help = "Remove extended attribute")]
    remove: Option<String>,
    #[command(flatten)]
    transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for SetXattrCommand {
    #[inline]
    fn execute(self) -> io::Result<()> {
        archive_set_xattr(self)
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
    #[inline]
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

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "text" => Ok(Self::Text),
            "hex" => Ok(Self::Hex),
            "base64" => Ok(Self::Base64),
            _ => Err("only allowed `text`, `hex` or `base64`".into()),
        }
    }
}

fn archive_get_xattr(args: GetXattrCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
    if args.files.is_empty() {
        return Ok(());
    }
    let globs = GlobPatterns::new(args.files)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let encoding = args.encoding;

    run_entries(
        &args.archive,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            let name = entry.header().path();
            if globs.matches_any(name) {
                println!("# file: {}", name);
                for attr in entry.xattrs().iter().filter(|a| {
                    args.name.is_none() || args.name.as_deref().is_some_and(|it| it == a.name())
                }) {
                    match encoding {
                        None => {
                            println!("{}={}", attr.name(), DisplayAuto(attr.value()));
                        }
                        Some(Encoding::Text) => {
                            println!("{}={}", attr.name(), DisplayText(attr.value()));
                        }
                        Some(Encoding::Hex) => {
                            println!("{}={}", attr.name(), DisplayHex(attr.value()));
                        }
                        Some(Encoding::Base64) => {
                            println!("{}={}", attr.name(), DisplayBase64(attr.value()));
                        }
                    }
                }
            }
            Ok(())
        },
    )?;
    Ok(())
}

fn archive_set_xattr(args: SetXattrCommand) -> io::Result<()> {
    let password = ask_password(args.password)?;
    if args.files.is_empty() {
        return Ok(());
    }
    let globs = GlobPatterns::new(args.files)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let value = args
        .value
        .as_ref()
        .map_or_else(Default::default, |it| it.as_bytes());

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => run_transform_entry(
            args.archive.remove_part().unwrap(),
            &args.archive,
            || password.as_deref(),
            |entry| {
                let entry = entry?;
                if globs.matches_any(entry.header().path()) {
                    Ok(Some(transform_entry(
                        entry,
                        args.name.as_deref(),
                        value,
                        args.remove.as_deref(),
                    )))
                } else {
                    Ok(Some(entry))
                }
            },
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => run_transform_entry(
            args.archive.remove_part().unwrap(),
            &args.archive,
            || password.as_deref(),
            |entry| {
                let entry = entry?;
                if globs.matches_any(entry.header().path()) {
                    Ok(Some(transform_entry(
                        entry,
                        args.name.as_deref(),
                        value,
                        args.remove.as_deref(),
                    )))
                } else {
                    Ok(Some(entry))
                }
            },
            TransformStrategyKeepSolid,
        ),
    }
}

#[inline]
fn transform_entry<T>(
    entry: NormalEntry<T>,
    name: Option<&str>,
    value: &[u8],
    remove: Option<&str>,
) -> NormalEntry<T> {
    let mut xattrs = entry
        .xattrs()
        .iter()
        .map(|it| (it.name(), it.value()))
        .collect::<IndexMap<_, _>>();
    if let Some(name) = name {
        let map_entry = xattrs.entry(name);
        map_entry.or_insert(value);
    }
    if let Some(name) = remove {
        xattrs.shift_remove_entry(name);
    }
    let xattrs = xattrs
        .into_iter()
        .map(|(key, value)| pna::ExtendedAttribute::new(key.into(), value.into()))
        .collect::<Vec<_>>();
    entry.with_xattrs(&xattrs)
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct Value(Vec<u8>);

impl FromStr for Value {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(if let Some(stripped) = s.strip_prefix("0x") {
            char_chunks(stripped, 2)
                .map(|i| u8::from_str_radix(i, 16))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        } else if let Some(stripped) = s.strip_prefix("0s") {
            base64::engine::general_purpose::STANDARD
                .decode(stripped)
                .map_err(|e| e.to_string())?
        } else {
            s.into()
        }))
    }
}

impl Value {
    #[inline]
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

struct DisplayAuto<'a>(&'a [u8]);

impl Display for DisplayAuto<'_> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match std::str::from_utf8(self.0) {
            Ok(s) => {
                f.write_char('"')?;
                Display::fmt(&EscapeXattrValueText(s), f)?;
                f.write_char('"')
            }
            Err(_e) => Display::fmt(&DisplayHex(self.0), f),
        }
    }
}

struct DisplayText<'a>(&'a [u8]);

impl Display for DisplayText<'_> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match std::str::from_utf8(self.0) {
            Ok(s) => {
                f.write_char('"')?;
                Display::fmt(&EscapeXattrValueText(s), f)?;
                f.write_char('"')
            }
            Err(e) => Display::fmt(&e, f),
        }
    }
}

struct DisplayHex<'a>(&'a [u8]);

impl Display for DisplayHex<'_> {
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

impl Display for DisplayBase64<'_> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("0s")?;
        f.write_str(&base64::engine::general_purpose::STANDARD.encode(self.0))
    }
}

struct EscapeXattrValueText<'s>(&'s str);

impl Display for EscapeXattrValueText<'_> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.chars().try_for_each(|c| match c {
            '"' => f.write_str("\\\""),
            '\\' => f.write_str("\\\\"),
            _ => f.write_char(c),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_text() {
        let v = DisplayText(b"abc");
        assert_eq!(format!("{}", v), "\"abc\"");
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

    #[test]
    fn decode_text() {
        assert_eq!(Value(b"abc".into()), Value::from_str("abc").unwrap());
    }

    #[test]
    fn decode_hex() {
        assert_eq!(Value(b"abc".into()), Value::from_str("0x616263").unwrap());
    }

    #[test]
    fn decode_base64() {
        assert_eq!(Value(b"abc".into()), Value::from_str("0sYWJj").unwrap());
    }

    #[test]
    fn escape_text() {
        assert_eq!("", format!("{}", EscapeXattrValueText("")));
        assert_eq!("a\\\\b\\\"", format!("{}", EscapeXattrValueText("a\\b\"")));
    }
}
