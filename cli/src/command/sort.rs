use crate::{
    cli::PasswordArgs,
    command::{
        Command, ask_password,
        core::{collect_split_archives, run_entries},
    },
    utils::{PathPartExt, env::NamedTempFile},
};
use clap::{Parser, ValueHint};
use pna::{Archive, NormalEntry};
use std::{
    fmt::{self, Display, Formatter},
    path::PathBuf,
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
    #[arg(short = 'f', long = "file", value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(long, help = "Output file path", value_hint = ValueHint::FilePath)]
    output: Option<PathBuf>,
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
    fn execute(self) -> anyhow::Result<()> {
        sort_archive(self)
    }
}

fn sort_archive(args: SortCommand) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    let archives = collect_split_archives(&args.archive)?;
    #[cfg(feature = "memmap")]
    let mmaps = archives
        .into_iter()
        .map(crate::utils::mmap::Mmap::try_from)
        .collect::<std::io::Result<Vec<_>>>()?;
    #[cfg(feature = "memmap")]
    let archives = mmaps.iter().map(|m| m.as_ref());
    let mut entries = Vec::<NormalEntry<_>>::new();
    run_entries(
        archives,
        || password.as_deref(),
        |entry| {
            entries.push(entry?);
            Ok(())
        },
    )?;

    entries.sort_by(|a, b| {
        for key in &args.by {
            let ord = match key.by {
                SortBy::Name => a.header().path().cmp(b.header().path()),
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

    let mut temp_file =
        NamedTempFile::new(|| args.archive.parent().unwrap_or_else(|| ".".as_ref()))?;
    let mut archive = Archive::write_header(temp_file.as_file_mut())?;
    for entry in entries {
        archive.add_entry(entry)?;
    }
    archive.finalize()?;

    #[cfg(feature = "memmap")]
    drop(mmaps);

    let output = args.output.unwrap_or_else(|| args.archive.remove_part());
    temp_file.persist(output)?;

    Ok(())
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
