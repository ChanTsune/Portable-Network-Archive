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
    utils::{env::NamedTempFile, fmt::hex, GlobPatterns, PathPartExt},
};
use base64::Engine;
use bstr::{io::BufReadExt, ByteSlice};
use clap::{ArgGroup, Parser, ValueEnum, ValueHint};
use indexmap::IndexMap;
use pna::NormalEntry;
use regex::Regex;
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter, Write},
    fs, io,
    num::ParseIntError,
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
    fn execute(self) -> anyhow::Result<()> {
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
    #[arg(
        short = 'm',
        long = "match",
        value_name = "pattern",
        help = "Only include attributes with names matching the regular expression pattern. Specify '-' for including all attributes"
    )]
    regex_match: Option<String>,
    #[arg(short, long, help = "Encode values after retrieving them")]
    encoding: Option<Encoding>,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for GetXattrCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
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
    #[arg(
        long,
        help = "Restores extended attributes from file. The file must be in the format generated by the pna xattr get command with the --dump option. If a dash (-) is given as the file name, reads from standard input",
        value_hint = ValueHint::FilePath
    )]
    restore: Option<String>,
    #[command(flatten)]
    transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for SetXattrCommand {
    #[inline]
    fn execute(self) -> anyhow::Result<()> {
        archive_set_xattr(self)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default, ValueEnum)]
#[value(rename_all = "lower")]
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

enum MatchStrategy<'s> {
    All,
    Named(&'s str),
    Regex(Regex),
}

struct DumpOption<'s> {
    dump: bool,
    matcher: MatchStrategy<'s>,
}

impl<'a> DumpOption<'a> {
    #[inline]
    fn new(
        dump: bool,
        name: Option<&'a str>,
        regex_match: Option<&'a str>,
    ) -> Result<Self, regex::Error> {
        Ok(match (name, regex_match) {
            (None, None) | (None, Some("-")) => Self {
                dump,
                matcher: MatchStrategy::All,
            },
            (None, Some(re)) => Self {
                dump,
                matcher: MatchStrategy::Regex(Regex::new(re)?),
            },
            (Some(name), _) => Self {
                dump: true,
                matcher: MatchStrategy::Named(name),
            },
        })
    }

    #[inline]
    fn is_match(&self, name: &str) -> bool {
        match self.matcher {
            MatchStrategy::All => true,
            MatchStrategy::Named(n) => n == name,
            MatchStrategy::Regex(ref re) => re.is_match(name),
        }
    }
}

fn archive_get_xattr(args: GetXattrCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    if args.files.is_empty() {
        return Ok(());
    }
    let mut globs = GlobPatterns::new(args.files.iter().map(|p| p.as_str()))?;
    let encoding = args.encoding;
    let dump_option = DumpOption::new(args.dump, args.name.as_deref(), args.regex_match.as_deref())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let archives = collect_split_archives(&args.archive)?;

    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(crate::utils::mmap::Mmap::try_from)
        .collect::<io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());

    run_entries(
        archives,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            let name = entry.header().path();
            if globs.matches_any(name) {
                println!("# file: {name}");
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
                println!();
            }
            Ok(())
        },
    )?;
    globs.ensure_all_matched()?;
    Ok(())
}

fn archive_set_xattr(args: SetXattrCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let mut set_strategy = if let Some("-") = args.restore.as_deref() {
        SetAttrStrategy::Restore(parse_dump(io::stdin().lock())?)
    } else if let Some(path) = args.restore.as_deref() {
        SetAttrStrategy::Restore(parse_dump(io::BufReader::new(fs::File::open(path)?))?)
    } else if args.files.is_empty() {
        return Ok(());
    } else {
        let globs = GlobPatterns::new(args.files.iter().map(|p| p.as_str()))?;
        let value = args.value.unwrap_or_default();
        SetAttrStrategy::Apply {
            globs,
            name: args.name,
            value,
            remove: args.remove,
        }
    };

    let archives = collect_split_archives(&args.archive)?;

    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(crate::utils::mmap::Mmap::try_from)
        .collect::<io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());

    let output_path = args.archive.remove_part().unwrap();
    let mut temp_file =
        NamedTempFile::new(|| output_path.parent().unwrap_or_else(|| ".".as_ref()))?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => run_transform_entry(
            temp_file.as_file_mut(),
            archives,
            || password.as_deref(),
            |entry| Ok(Some(set_strategy.transform_entry(entry?))),
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => run_transform_entry(
            temp_file.as_file_mut(),
            archives,
            || password.as_deref(),
            |entry| Ok(Some(set_strategy.transform_entry(entry?))),
            TransformStrategyKeepSolid,
        ),
    }?;

    #[cfg(feature = "memmap")]
    drop(mmaps);

    temp_file.persist(output_path)?;

    if let SetAttrStrategy::Apply { globs, .. } = set_strategy {
        globs.ensure_all_matched()?;
    }
    Ok(())
}

enum SetAttrStrategy<'s> {
    Restore(HashMap<String, Vec<(String, Value)>>),
    Apply {
        globs: GlobPatterns<'s>,
        name: Option<String>,
        value: Value,
        remove: Option<String>,
    },
}

impl SetAttrStrategy<'_> {
    #[inline]
    fn transform_entry<T>(&mut self, entry: NormalEntry<T>) -> NormalEntry<T> {
        match self {
            SetAttrStrategy::Restore(restore) => {
                if let Some(attrs) = restore.get(entry.header().path().as_str()) {
                    let xattrs = entry
                        .xattrs()
                        .iter()
                        .map(|it| (it.name(), it.value()))
                        .chain(attrs.iter().map(|(k, v)| (k.as_str(), v.as_bytes())))
                        .collect::<IndexMap<_, _>>();
                    let xattrs = xattrs
                        .into_iter()
                        .map(|(key, value)| pna::ExtendedAttribute::new(key.into(), value.into()))
                        .collect::<Vec<_>>();
                    entry.with_xattrs(xattrs)
                } else {
                    entry
                }
            }
            SetAttrStrategy::Apply {
                globs,
                name,
                value,
                remove,
            } => {
                if globs.matches_any(entry.header().path()) {
                    transform_entry(entry, name.as_deref(), value.as_bytes(), remove.as_deref())
                } else {
                    entry
                }
            }
        }
    }
}

fn parse_dump(reader: impl io::BufRead) -> io::Result<HashMap<String, Vec<(String, Value)>>> {
    let mut result = HashMap::<_, Vec<_>>::new();
    let mut current_file = None;

    for line in reader.byte_lines() {
        let line = line?;
        if let Some(path) = line.strip_prefix(b"# file: ") {
            current_file = Some(
                String::from_utf8(path.into())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            );
        } else if let Some(file) = &current_file {
            // TODO: use slice::split_once when it is stabilized.
            if let Some((key, value)) = line.split_once_str("=") {
                let key = String::from_utf8(key.into())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                let value = Value::try_from(value)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                result.entry(file.clone()).or_default().push((key, value));
            }
        }
    }
    Ok(result)
}

#[inline]
fn transform_entry<T>(
    entry: NormalEntry<T>,
    name: Option<&str>,
    value: &[u8],
    remove: Option<&str>,
) -> NormalEntry<T> {
    let xattrs = transform_xattr(entry.xattrs(), name, value, remove);
    entry.with_xattrs(xattrs)
}

#[inline]
fn transform_xattr(
    xattrs: &[pna::ExtendedAttribute],
    name: Option<&str>,
    value: &[u8],
    remove: Option<&str>,
) -> Vec<pna::ExtendedAttribute> {
    let mut xattrs = xattrs
        .iter()
        .map(|it| (it.name(), it.value()))
        .collect::<IndexMap<_, _>>();
    if let Some(name) = name {
        xattrs.insert(name, value);
    }
    if let Some(name) = remove {
        xattrs.shift_remove_entry(name);
    }
    xattrs
        .into_iter()
        .map(|(key, value)| pna::ExtendedAttribute::new(key.into(), value.into()))
        .collect()
}

#[derive(thiserror::Error, Clone, Eq, PartialEq, Debug)]
enum ValueError {
    #[error(transparent)]
    InvalidHex(#[from] ParseIntError),
    #[error(transparent)]
    InvalidBase64(#[from] base64::DecodeError),
    #[error("missing tailing quote")]
    Unclosed,
    #[error("unknown escape character")]
    InvalidEscaped,
}

#[derive(Clone, Default, Eq, PartialEq, Hash, Debug)]
struct Value(Vec<u8>);

impl FromStr for Value {
    type Err = ValueError;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.as_bytes())
    }
}

impl TryFrom<&[u8]> for Value {
    type Error = ValueError;

    #[inline]
    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self(if let Some(stripped) = s.strip_prefix(b"0x") {
            stripped
                .chunks(2)
                .map(|i| u8::from_str_radix(unsafe { std::str::from_utf8_unchecked(i) }, 16))
                .collect::<Result<Vec<_>, _>>()?
        } else if let Some(stripped) = s.strip_prefix(b"0s") {
            base64::engine::general_purpose::STANDARD.decode(stripped)?
        } else if let Some(s) = s.strip_prefix(b"\"") {
            if s.ends_with(b"\\\"") && !s.ends_with(b"\\\\\"") {
                return Err(ValueError::Unclosed);
            } else if let Some(s) = s.strip_suffix(b"\"") {
                unescape_xattr_value_text(s)?
            } else {
                return Err(ValueError::Unclosed);
            }
        } else {
            s.to_vec()
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
            Ok(_) => self.fmt_text(f),
            Err(_e) => self.fmt_base64(f),
        }
    }

    #[inline]
    fn fmt_text(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_char('"')?;
        Display::fmt(
            &unsafe { String::from_utf8_unchecked(escape_xattr_value_text(self.value)) },
            f,
        )?;
        f.write_char('"')
    }

    #[inline]
    fn fmt_hex(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("0x")?;
        Display::fmt(&hex::display(self.value), f)
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

fn escape_xattr_value_text(text: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(text.len());
    text.iter().for_each(|c| match c {
        b'"' => result.extend_from_slice(b"\\\""),
        b'\\' => result.extend_from_slice(b"\\\\"),
        b'\0' | b'\n' | b'\r' => {
            result.push(b'\\');
            result.push(b'0' + (*c >> 6));
            result.push(b'0' + ((*c & 0o70) >> 3));
            result.push(b'0' + (*c & 0o7));
        }
        _ => result.push(*c),
    });
    result
}

fn unescape_xattr_value_text(text: &[u8]) -> Result<Vec<u8>, ValueError> {
    let mut result = Vec::with_capacity(text.len());
    let mut chars = text.iter().copied();
    while let Some(c) = chars.next() {
        match c {
            b'\\' => {
                if let Some(next_char) = chars.next() {
                    if next_char == b'\\' {
                        result.push(b'\\')
                    } else if next_char == b'"' {
                        result.push(b'"')
                    } else if matches!(next_char, b'0'..=b'7') {
                        let mut unescaped = next_char - b'0';
                        let next_char = chars.next().ok_or(ValueError::InvalidEscaped)?;
                        if matches!(next_char, b'0'..=b'7') {
                            unescaped = (unescaped << 3) + next_char - b'0';
                        }
                        let next_char = chars.next().ok_or(ValueError::InvalidEscaped)?;
                        if matches!(next_char, b'0'..=b'7') {
                            unescaped = (unescaped << 3) + next_char - b'0';
                        }
                        result.push(unescaped);
                    } else {
                        return Err(ValueError::InvalidEscaped);
                    }
                } else {
                    return Err(ValueError::InvalidEscaped);
                }
            }
            _ => result.push(c),
        };
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dump_for_restore() {
        assert_eq!(
            parse_dump(
                [
                    "# file: path/to/file1",
                    "user.a=\"abc\"",
                    "user.b=0x0102",
                    "",
                    "# file: path/to/file2",
                    "user.c=0sYWJj",
                ]
                .join("\n")
                .as_bytes()
            )
            .unwrap(),
            maplit::hashmap! {
                "path/to/file1".into() =>
                vec![
                    ("user.a".into(), Value("abc".into())),
                    ("user.b".into(), Value(vec![1, 2])),
                ],
                "path/to/file2".into() =>
                vec![("user.c".into(), Value("abc".into()))],

            }
        );
    }

    #[test]
    fn encode_text() {
        let v = DisplayValue::new(b"abc", Some(Encoding::Text));
        assert_eq!(format!("{v}"), "\"abc\"");
    }

    #[test]
    fn encode_hex() {
        let v = DisplayValue::new(b"abc", Some(Encoding::Hex));
        assert_eq!(format!("{v}"), "0x616263");
    }

    #[test]
    fn encode_base64() {
        let v = DisplayValue::new(b"abc", Some(Encoding::Base64));
        assert_eq!(format!("{v}"), "0sYWJj");
    }

    #[test]
    fn decode_text() {
        assert_eq!(Value(b"abc".into()), Value::from_str("abc").unwrap());
        assert_eq!(Value(b"a\\".into()), Value::from_str("\"a\\\\\"").unwrap());
        assert_eq!(Value(b"".into()), Value::from_str("").unwrap());
    }

    #[test]
    fn decode_hex() {
        assert_eq!(Value(b"abc".into()), Value::from_str("0x616263").unwrap());
        assert_eq!(
            Value([0, 1, 17].into()),
            Value::from_str("0x000111").unwrap()
        );
        assert_eq!(Value(b"".into()), Value::from_str("0x").unwrap());
    }

    #[test]
    fn decode_base64() {
        assert_eq!(Value(b"abc".into()), Value::from_str("0sYWJj").unwrap());
        assert_eq!(Value(b"".into()), Value::from_str("0s").unwrap());
    }

    #[test]
    fn escape_text() {
        assert_eq!(b"".as_slice(), escape_xattr_value_text(b""));
        assert_eq!(b"a\\\\b\\\"".as_slice(), escape_xattr_value_text(b"a\\b\""));
    }

    #[test]
    fn escape_unescape() {
        assert_eq!(
            b"\"\\\n\r\0".as_slice(),
            unescape_xattr_value_text(&escape_xattr_value_text(b"\"\\\n\r\0")).unwrap()
        );
    }

    #[test]
    fn set_xattr() {
        let xattrs = transform_xattr(&[], Some("key"), b"value", None);

        assert_eq!(
            xattrs,
            vec![pna::ExtendedAttribute::new("key".into(), b"value".into()),]
        );
    }

    #[test]
    fn overwrite_xattr() {
        let xattrs = transform_xattr(
            &[pna::ExtendedAttribute::new("key".into(), b"origin".into())],
            Some("key"),
            b"value",
            None,
        );

        assert_eq!(
            xattrs,
            vec![pna::ExtendedAttribute::new("key".into(), b"value".into()),]
        );
    }

    #[test]
    fn remove_xattr() {
        let xattrs = transform_xattr(
            &[pna::ExtendedAttribute::new("key".into(), b"origin".into())],
            None,
            b"value",
            Some("key"),
        );

        assert_eq!(xattrs, vec![]);
    }
}
