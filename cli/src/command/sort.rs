use crate::{
    cli::{ArchiveFileArgs, ArchiveOutputArgs, PasswordArgs},
    command::{
        Command, ask_password,
        core::{
            EntryVisitor, Umask,
            archive_destination::{SinkConsumer, resolve_transform_destination},
        },
    },
};
use clap::Parser;
use pna::{Archive, NormalEntry, ReadOptions};
use std::{
    borrow::Cow,
    fmt::{self, Display, Formatter},
    io,
    str::FromStr,
};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum SortBy {
    Name,
    Ctime,
    Mtime,
    Atime,
}

impl Display for SortBy {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SortBy::Name => "name",
            SortBy::Ctime => "ctime",
            SortBy::Mtime => "mtime",
            SortBy::Atime => "atime",
        })
    }
}

impl FromStr for SortBy {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "name" => Ok(Self::Name),
            "ctime" => Ok(Self::Ctime),
            "mtime" => Ok(Self::Mtime),
            "atime" => Ok(Self::Atime),
            _ => Err("allowed values: name, ctime, mtime, atime".into()),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum SortOrder {
    Asc,
    Desc,
}

impl Display for SortOrder {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            SortOrder::Asc => "asc",
            SortOrder::Desc => "desc",
        })
    }
}

impl FromStr for SortOrder {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "asc" => Ok(Self::Asc),
            "desc" => Ok(Self::Desc),
            _ => Err("only allowed `asc` or `desc`".into()),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct SortKey {
    by: SortBy,
    order: SortOrder,
}

impl Default for SortKey {
    fn default() -> Self {
        Self {
            by: SortBy::Name,
            order: SortOrder::Asc,
        }
    }
}

impl Display for SortKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.order == SortOrder::Asc {
            write!(f, "{}", self.by)
        } else {
            write!(f, "{}:{}", self.by, self.order)
        }
    }
}

impl FromStr for SortKey {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (by, order) = match s.split_once(':') {
            None => (s, SortOrder::Asc),
            Some((b, "")) => (b, SortOrder::Asc),
            Some((b, o)) => (b, SortOrder::from_str(o)?),
        };
        let by = SortBy::from_str(by)?;
        Ok(Self { by, order })
    }
}

#[derive(Parser, Clone, Eq, PartialEq, Hash, Debug)]
pub(crate) struct SortCommand {
    #[command(flatten)]
    archive: ArchiveFileArgs,
    #[command(flatten)]
    output: ArchiveOutputArgs,
    #[arg(
        long = "by",
        value_name = "KEY",
        num_args = 1..,
        default_values_t = [SortKey::default()],
        help = "Sort key in format KEY[:ORDER] (e.g., name, mtime:desc) [keys: name, ctime, mtime, atime] [orders: asc, desc]"
    )]
    by: Vec<SortKey>,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for SortCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        sort_archive(self, ctx.umask())
    }
}

/// Collects every entry in memory; sorting needs the complete set before anything is written.
#[derive(Default)]
struct OwnedEntries(Vec<NormalEntry<Vec<u8>>>);

impl EntryVisitor for OwnedEntries {
    fn visit(&mut self, entry: NormalEntry<Cow<'_, [u8]>>) -> io::Result<()> {
        self.0.push(entry.into());
        Ok(())
    }
}

#[hooq::hooq(anyhow)]
fn sort_archive(args: SortCommand, umask: Umask) -> anyhow::Result<()> {
    let source_arg = args.archive.source();
    let destination =
        resolve_transform_destination(&source_arg, args.output.output, args.output.overwrite)?;
    let password = ask_password(args.password)?;
    let read_options = ReadOptions::with_password(password.as_deref());
    let mut entries = OwnedEntries::default();
    source_arg
        .open()?
        .for_each_entry(&read_options, &mut entries)?;
    let OwnedEntries(mut entries) = entries;

    entries.sort_by(|a, b| {
        for key in &args.by {
            let ord = match key.by {
                SortBy::Name => a.name().cmp(b.name()),
                SortBy::Ctime => a.metadata().created().cmp(&b.metadata().created()),
                SortBy::Mtime => a.metadata().modified().cmp(&b.metadata().modified()),
                SortBy::Atime => a.metadata().accessed().cmp(&b.metadata().accessed()),
            };
            if ord != std::cmp::Ordering::Equal {
                return match key.order {
                    SortOrder::Asc => ord,
                    SortOrder::Desc => ord.reverse(),
                };
            }
        }
        std::cmp::Ordering::Equal
    });

    destination.open_with(umask, WriteSorted(entries))?;

    Ok(())
}

/// Writes the sorted entries as a fresh archive into the opened destination.
struct WriteSorted<T>(Vec<NormalEntry<T>>);

impl<T> SinkConsumer for WriteSorted<T>
where
    NormalEntry<T>: pna::prelude::Entry,
{
    type Output = ();

    fn consume<W: io::Write>(self, writer: W) -> anyhow::Result<()> {
        let mut archive = Archive::write_header(writer)?;
        for entry in self.0 {
            archive.add_entry(entry)?;
        }
        archive.finalize()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sort_key_default_order() {
        assert_eq!(
            SortKey::from_str("name").unwrap(),
            SortKey {
                by: SortBy::Name,
                order: SortOrder::Asc,
            }
        );
        assert_eq!(
            SortKey::from_str("name:").unwrap(),
            SortKey {
                by: SortBy::Name,
                order: SortOrder::Asc,
            }
        );
    }

    #[test]
    fn parse_sort_key_explicit_orders() {
        assert_eq!(
            SortKey::from_str("name:asc").unwrap(),
            SortKey {
                by: SortBy::Name,
                order: SortOrder::Asc,
            }
        );
        assert_eq!(
            SortKey::from_str("name:desc").unwrap(),
            SortKey {
                by: SortBy::Name,
                order: SortOrder::Desc,
            }
        );
    }

    #[test]
    fn parse_sort_key_invalid() {
        assert!(SortKey::from_str("name:foo").is_err());
        assert!(SortKey::from_str("foo").is_err());
    }
}
