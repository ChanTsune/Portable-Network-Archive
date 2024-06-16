#[cfg(any(unix, windows))]
use crate::utils::fs::{Group, User};
use crate::{
    cli::{PasswordArgs, Verbosity},
    command::{ask_password, commons::run_process_archive_path, Command},
    utils::{self, GlobPatterns, PathPartExt},
};
use clap::{Parser, ValueHint};
use pna::Archive;
use std::ops::Not;
use std::{env::temp_dir, fs, io, path::PathBuf, str::FromStr};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ChownCommand {
    #[arg(value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(help = "owner[:group]|:group")]
    owner: Owner,
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for ChownCommand {
    fn execute(self, verbosity: Verbosity) -> io::Result<()> {
        archive_chown(self, verbosity)
    }
}

fn archive_chown(args: ChownCommand, _: Verbosity) -> io::Result<()> {
    let password = ask_password(args.password)?;
    if args.files.is_empty() {
        return Ok(());
    }
    let globs = GlobPatterns::new(args.files)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let random = rand::random::<usize>();
    let outfile_path = temp_dir().join(format!("{}.pna.tmp", random));
    let outfile = fs::File::create(&outfile_path)?;
    let mut out_archive = Archive::write_header(outfile)?;

    run_process_archive_path(
        &args.archive,
        || password.as_deref(),
        |entry| {
            let entry = entry?;
            let name = entry.header().path().as_path();
            if globs.matches_any_path(name) {
                let metadata = entry.metadata().clone();
                let permission = metadata.permission().map(|p| {
                    let user = args.owner.user();
                    #[cfg(unix)]
                    let user = user.and_then(|it| {
                        User::from_name(it).map(|it| (it.0.uid.as_raw().into(), it.0.name))
                    });
                    #[cfg(windows)]
                    let user =
                        user.and_then(|it| User::from_name(it).map(|it| (u64::MAX, it.0.name)));
                    #[cfg(not(any(unix, windows)))]
                    let user = user.map(|_| (p.uid(), p.uname().into()));
                    let (uid, uname) = user.unwrap_or_else(|| (p.uid(), p.uname().into()));

                    let group = args.owner.group();
                    #[cfg(unix)]
                    let group = group.and_then(|it| {
                        Group::from_name(it).map(|it| (it.0.gid.as_raw().into(), it.0.name))
                    });
                    #[cfg(windows)]
                    let group =
                        group.and_then(|it| Group::from_name(it).map(|it| (u64::MAX, it.0.name)));
                    #[cfg(not(any(unix, windows)))]
                    let group = group.map(|_| (p.gid(), p.gname().into()));
                    let (gid, gname) = group.unwrap_or_else(|| (p.gid(), p.gname().into()));
                    pna::Permission::new(uid, uname, gid, gname, p.permissions())
                });
                out_archive.add_entry(entry.with_metadata(metadata.with_permission(permission)))?;
            } else {
                out_archive.add_entry(entry)?;
            }
            Ok(())
        },
    )?;

    out_archive.finalize()?;

    utils::fs::mv(outfile_path, args.archive.remove_part().unwrap())?;
    Ok(())
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct Owner {
    user: Option<String>,
    group: Option<String>,
}

impl Owner {
    #[inline]
    pub(crate) fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    #[inline]
    pub(crate) fn group(&self) -> Option<&str> {
        self.group.as_deref()
    }
}

impl FromStr for Owner {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() || s == ":" {
            return Err("owner must not be empty".into());
        }
        let (user, group) = if let Some((user, group)) = s.split_once(':') {
            (
                user.is_empty().not().then(|| user.into()),
                group.is_empty().not().then(|| group.into()),
            )
        } else {
            (Some(s.into()), None)
        };
        Ok(Self { user, group })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_from_str_user() {
        assert_eq!(
            Owner::from_str("user").unwrap(),
            Owner {
                user: Some("user".into()),
                group: None,
            }
        );
    }

    #[test]
    fn group() {
        assert_eq!(
            Owner::from_str(":group").unwrap(),
            Owner {
                user: None,
                group: Some("group".into()),
            }
        );
    }

    #[test]
    fn user_group() {
        assert_eq!(
            Owner::from_str("user:group").unwrap(),
            Owner {
                user: Some("user".into()),
                group: Some("group".into()),
            }
        );
    }
}
