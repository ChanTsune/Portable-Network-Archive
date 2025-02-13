use crate::{
    cli::{PasswordArgs, SolidEntriesTransformStrategy, SolidEntriesTransformStrategyArgs},
    command::{
        ask_password,
        commons::{
            collect_split_archives, run_entries, run_transform_entry, TransformStrategyKeepSolid,
            TransformStrategyUnSolid,
        },
        Command,
    },
    utils::{str::char_chunks, GlobPatterns, PathPartExt},
};
use base64::Engine;
use clap::{ArgGroup, Parser, ValueHint};
use indexmap::IndexMap;
use pna::NormalEntry;
use std::{
    fmt::{self, Display, Formatter, Write},
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
#[command(
    group(ArgGroup::new("dump-flags").args(["name", "dump"])),
)]
pub(crate) struct GetXattrCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[arg(short, long, help = "Dump the value of the named extended attribute")]
    name: Option<String>,
    #[arg(
        short,
        long,
        help = "Dump the values of all matched extended attributes"
    )]
    dump: bool,
    #[arg(short, long, help = "Encode values after retrieving them")]
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

enum MatchStrategy<'s> {
    All,
    Named(&'s str),
}

struct DumpOption<'s> {
    dump: bool,
    matcher: MatchStrategy<'s>,
}

impl<'a> DumpOption<'a> {
    #[inline]
    fn new(dump: bool, name: Option<&'a str>) -> Self {
        match name {
            Some(name) => Self {
                dump: true,
                matcher: MatchStrategy::Named(name),
            },
            None => Self {
                dump,
                matcher: MatchStrategy::All,
            },
        }
    }

    #[inline]
    fn is_match(&self, name: &str) -> bool {
        match self.matcher {
            MatchStrategy::All => true,
            MatchStrategy::Named(n) => n == name,
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
    let dump_option = DumpOption::new(args.dump, args.name.as_deref());

    let archives = collect_split_archives(&args.archive)?;

    run_entries(
        archives,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            let name = entry.header().path();
            if globs.matches_any(name) {
                println!("# file: {}", name);
                for attr in entry
                    .xattrs()
                    .iter()
                    .filter(|a| dump_option.is_match(a.name()))
                {
                    if dump_option.dump {
                        println!(
                            "{}={}",
                            attr.name(),
                            DisplayValue::new(attr.value(), encoding)
                        );
                    } else {
                        println!("{}", attr.name());
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

    let archives = collect_split_archives(&args.archive)?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => run_transform_entry(
            args.archive.remove_part().unwrap(),
            archives,
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
            archives,
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

struct DisplayValue<'a> {
    value: &'a [u8],
    encoding: Option<Encoding>,
}

impl<'a> DisplayValue<'a> {
    #[inline]
    const fn new(value: &'a [u8], encoding: Option<Encoding>) -> Self {
        Self { value, encoding }
    }

    #[inline]
    fn fmt_auto(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match std::str::from_utf8(self.value) {
            Ok(s) => {
                f.write_char('"')?;
                Display::fmt(&EscapeXattrValueText(s), f)?;
                f.write_char('"')
            }
            Err(_e) => self.fmt_hex(f),
        }
    }

    #[inline]
    fn fmt_text(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match std::str::from_utf8(self.value) {
            Ok(s) => {
                f.write_char('"')?;
                Display::fmt(&EscapeXattrValueText(s), f)?;
                f.write_char('"')
            }
            Err(e) => Display::fmt(&e, f),
        }
    }

    #[inline]
    fn fmt_hex(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("0x")?;
        for i in self.value {
            write!(f, "{:x}", i)?;
        }
        Ok(())
    }

    #[inline]
    fn fmt_base64(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("0s")?;
        f.write_str(&base64::engine::general_purpose::STANDARD.encode(self.value))
    }
}

impl Display for DisplayValue<'_> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.encoding {
            None => self.fmt_auto(f),
            Some(Encoding::Text) => self.fmt_text(f),
            Some(Encoding::Hex) => self.fmt_hex(f),
            Some(Encoding::Base64) => self.fmt_base64(f),
        }
    }
}

struct EscapeXattrValueText<'s>(&'s str);

impl Display for EscapeXattrValueText<'_> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
        let v = DisplayValue::new(b"abc", Some(Encoding::Text));
        assert_eq!(format!("{}", v), "\"abc\"");
    }

    #[test]
    fn encode_hex() {
        let v = DisplayValue::new(b"abc", Some(Encoding::Hex));
        assert_eq!(format!("{}", v), "0x616263");
    }

    #[test]
    fn encode_base64() {
        let v = DisplayValue::new(b"abc", Some(Encoding::Base64));
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
