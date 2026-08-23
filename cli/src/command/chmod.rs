use crate::{
    cli::{
        ArchiveFileArgs, PasswordArgs, SolidEntriesTransformStrategy,
        SolidEntriesTransformStrategyArgs,
    },
    command::{
        Command, ask_password,
        core::{
            SplitArchiveReader, StagedArchive, TransformStrategyKeepSolid,
            TransformStrategyUnSolid, Umask, collect_split_archives,
        },
    },
    utils::{GlobPatterns, PathPartExt},
};
use bitflags::bitflags;
use clap::{Parser, ValueHint};
use nom::{
    Parser as _,
    branch::alt,
    character::complete::char,
    combinator::{map, opt},
    multi::{many0, many1, separated_list1},
};
use pna::{DataKind, NormalEntry};
use std::{ops::BitOr, str::FromStr};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ChmodCommand {
    #[command(flatten)]
    archive: ArchiveFileArgs,
    #[arg(help = "mode")]
    mode: Mode,
    #[arg(value_hint = ValueHint::AnyPath)]
    files: Vec<String>,
    #[command(flatten)]
    transform_strategy: SolidEntriesTransformStrategyArgs,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for ChmodCommand {
    #[inline]
    fn execute(self, ctx: &crate::cli::GlobalContext) -> anyhow::Result<()> {
        archive_chmod(self, ctx.umask())
    }
}

#[hooq::hooq(anyhow)]
fn archive_chmod(args: ChmodCommand, umask: Umask) -> anyhow::Result<()> {
    let password = ask_password(args.password)?;
    if args.files.is_empty() {
        return Ok(());
    }
    let mut globs = GlobPatterns::new(args.files.iter().map(|p| p.as_str()))?;

    let mut source = SplitArchiveReader::new(collect_split_archives(&args.archive.file)?)?;

    let output_path = args.archive.file.remove_part();
    let mut staged = StagedArchive::new(output_path, umask)?;

    match args.transform_strategy.strategy() {
        SolidEntriesTransformStrategy::UnSolid => source.transform_entries(
            staged.as_file_mut(),
            password.as_deref(),
            #[hooq::skip_all]
            |entry| {
                let entry = entry?;
                if globs.matches_any(entry.name()) {
                    Ok(Some(transform_entry(entry, &args.mode)))
                } else {
                    Ok(Some(entry))
                }
            },
            TransformStrategyUnSolid,
        ),
        SolidEntriesTransformStrategy::KeepSolid => source.transform_entries(
            staged.as_file_mut(),
            password.as_deref(),
            #[hooq::skip_all]
            |entry| {
                let entry = entry?;
                if globs.matches_any(entry.name()) {
                    Ok(Some(transform_entry(entry, &args.mode)))
                } else {
                    Ok(Some(entry))
                }
            },
            TransformStrategyKeepSolid,
        ),
    }?;

    drop(source);

    staged.commit(Some(&globs))?;
    Ok(())
}

#[inline]
#[allow(deprecated)]
fn transform_entry<T>(entry: NormalEntry<T>, mode: &Mode) -> NormalEntry<T> {
    let metadata = entry.metadata().clone();
    let own = crate::ext::ResolvedOwnership::from_metadata(&metadata);
    let cur_mode = own
        .mode
        .unwrap_or_else(|| default_permission_mode(entry.header().data_kind()));
    let new_mode = mode.apply_to(cur_mode);
    let metadata = metadata
        .with_permission(None)
        .with_owner_uid(own.uid.map(pna::OwnerUid::from))
        .with_owner_gid(own.gid.map(pna::OwnerGid::from))
        .with_owner_user_name(
            own.uname
                .as_deref()
                .and_then(crate::command::core::permission::owner_name_opt),
        )
        .with_owner_group_name(
            own.gname
                .as_deref()
                .and_then(crate::command::core::permission::owner_group_name_opt),
        )
        .with_owner_user_sid(
            own.user_sid
                .clone()
                .map(|s| pna::OwnerUserSid::new(s).expect("rescued sid within owner-facet bound")),
        )
        .with_owner_group_sid(
            own.group_sid
                .clone()
                .map(|s| pna::OwnerGroupSid::new(s).expect("rescued sid within owner-facet bound")),
        )
        .with_permission_mode(Some(pna::PermissionMode::from(new_mode)));
    entry.with_metadata(metadata)
}

#[inline]
const fn default_permission_mode(kind: DataKind) -> u16 {
    match kind {
        DataKind::DIRECTORY => 0o755,
        DataKind::FILE | DataKind::SYMBOLIC_LINK | DataKind::HARD_LINK => 0o644,
        _ => 0,
    }
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub(crate) struct Who: u8 {
        const User = 0b001;
        const Group = 0b010;
        const Other = 0b100;
        const All = 0b111;
    }
}

impl Who {
    /// Expand 3-bit permission value to positions specified by this Who mask.
    #[inline]
    const fn to_permission_bits(self, n: u16) -> u16 {
        let mut result = 0;
        if self.contains(Who::User) {
            result |= n << 6;
        }
        if self.contains(Who::Group) {
            result |= n << 3;
        }
        if self.contains(Who::Other) {
            result |= n;
        }
        result
    }

    /// Extract 3-bit permission values from positions specified by this Who mask.
    #[inline]
    const fn extract_bits(self, mode: u16) -> u16 {
        let mut result = 0;
        if self.contains(Who::User) {
            result |= (mode >> 6) & 0o7;
        }
        if self.contains(Who::Group) {
            result |= (mode >> 3) & 0o7;
        }
        if self.contains(Who::Other) {
            result |= mode & 0o7;
        }
        result
    }

    #[inline]
    fn parse_from(s: &str) -> nom::IResult<&str, Who> {
        alt((
            map(char('a'), |_| Self::All),
            map(char('u'), |_| Self::User),
            map(char('g'), |_| Self::Group),
            map(char('o'), |_| Self::Other),
        ))
        .parse(s)
    }
}

/// Permission bits that can be either literal (r, w, x) or copied from another class (u, g, o).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct PermBits {
    /// Direct permission bits (r=4, w=2, x=1)
    literal: u8,
    /// Sources to copy from (resolved at apply time)
    copy_from: Who,
}

impl Default for PermBits {
    #[inline]
    fn default() -> Self {
        Self {
            literal: 0,
            copy_from: Who::empty(),
        }
    }
}

impl From<u8> for PermBits {
    #[inline]
    fn from(literal: u8) -> Self {
        Self {
            literal,
            copy_from: Who::empty(),
        }
    }
}

impl PermBits {
    /// Create a PermBits that copies from the given source(s).
    const fn copy_from(source: Who) -> Self {
        Self {
            literal: 0,
            copy_from: source,
        }
    }

    /// Resolve copy sources against current mode to get final permission bits.
    #[must_use]
    #[inline]
    const fn resolve(self, mode: u16) -> u16 {
        self.literal as u16 | self.copy_from.extract_bits(mode)
    }
}

impl BitOr for PermBits {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self {
            literal: self.literal | rhs.literal,
            copy_from: self.copy_from | rhs.copy_from,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Action {
    Equal(PermBits),
    Plus(PermBits),
    Minus(PermBits),
}

impl Action {
    #[inline]
    fn parse_from(s: &str) -> nom::IResult<&str, Self> {
        #[derive(Copy, Clone)]
        enum Op {
            Plus,
            Minus,
            Equal,
        }

        fn op(s: &str) -> nom::IResult<&str, Op> {
            alt((
                map(char('+'), |_| Op::Plus),
                map(char('-'), |_| Op::Minus),
                map(char('='), |_| Op::Equal),
            ))
            .parse(s)
        }

        fn perm(s: &str) -> nom::IResult<&str, PermBits> {
            alt((
                // Literal permission bits
                map(char('r'), |_| 0o4.into()),
                map(char('w'), |_| 0o2.into()),
                map(char('x'), |_| 0o1.into()),
                // Copy sources
                map(char('u'), |_| PermBits::copy_from(Who::User)),
                map(char('g'), |_| PermBits::copy_from(Who::Group)),
                map(char('o'), |_| PermBits::copy_from(Who::Other)),
            ))
            .parse(s)
        }

        map((op, many0(perm)), |(op, perms)| {
            let bits = perms.into_iter().fold(PermBits::default(), BitOr::bitor);
            match op {
                Op::Plus => Action::Plus(bits),
                Op::Minus => Action::Minus(bits),
                Op::Equal => Action::Equal(bits),
            }
        })
        .parse(s)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct ModeClause {
    who: Who,
    actions: Vec<Action>,
}

impl ModeClause {
    #[inline]
    fn parse_from(s: &str) -> nom::IResult<&str, Self> {
        map(
            (opt(many1(Who::parse_from)), many1(Action::parse_from)),
            |(who, actions)| ModeClause {
                who: who
                    .map(|w| w.into_iter().fold(Who::empty(), BitOr::bitor))
                    .unwrap_or(Who::All),
                actions,
            },
        )
        .parse(s)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) enum Mode {
    Numeric(u16),
    Clause(Vec<ModeClause>),
}

impl Mode {
    #[inline]
    pub(crate) fn apply_to(&self, mut mode: u16) -> u16 {
        match self {
            Mode::Numeric(mode) => *mode,
            Mode::Clause(clauses) => {
                for ModeClause { who, actions } in clauses {
                    for action in actions {
                        match action {
                            Action::Equal(bits) => {
                                // Resolve copy sources against current mode
                                let m = bits.resolve(mode);
                                let mask = who.to_permission_bits(0o7);
                                mode = (mode & !mask) | who.to_permission_bits(m);
                            }
                            Action::Plus(bits) => {
                                let m = bits.resolve(mode);
                                mode |= who.to_permission_bits(m)
                            }
                            Action::Minus(bits) => {
                                let m = bits.resolve(mode);
                                mode &= !who.to_permission_bits(m)
                            }
                        }
                    }
                }
                mode
            }
        }
    }
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.chars().all(|c| c.is_ascii_digit()) {
            return if s.len() == 3 {
                u16::from_str_radix(s, 8)
                    .map(Self::Numeric)
                    .map_err(|e| e.to_string())
            } else {
                Err(format!("Invalid mode length: {}", s.len()))
            };
        }
        separated_list1(char(','), ModeClause::parse_from)
            .parse_complete(s)
            .map_err(|e| e.to_string())
            .and_then(|(remain, mode)| {
                if remain.is_empty() {
                    Ok(Mode::Clause(mode))
                } else {
                    Err(format!("Invalid file mode: {s}"))
                }
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_modes(cases: &[(&str, u16, u16)]) {
        for &(input, initial, expected) in cases {
            let actual = Mode::from_str(input)
                .unwrap_or_else(|error| panic!("failed to parse {input:?}: {error}"))
                .apply_to(initial);
            assert_eq!(actual, expected, "mode {input:?} applied to {initial:#o}");
        }
    }

    #[test]
    fn numeric_modes_replace_the_existing_mode() {
        assert_modes(&[
            ("755", 0o764, 0o755),
            ("000", 0o777, 0o000),
            ("644", 0o000, 0o644),
        ]);
    }

    #[test]
    fn symbolic_modes_select_permission_classes() {
        assert_modes(&[
            ("+x", 0o664, 0o775),
            ("u=rw", 0o077, 0o677),
            ("ug=rw", 0o777, 0o667),
            ("go-x", 0o777, 0o766),
            ("a-w", 0o777, 0o555),
            ("u=,g=,o=", 0o777, 0o000),
        ]);
    }

    #[test]
    fn clauses_and_actions_apply_left_to_right() {
        assert_modes(&[
            ("u=rwx,g=rx,o=r", 0o000, 0o754),
            ("ug=rwx,o=rx", 0o000, 0o775),
            ("u=rw,g=r,o=", 0o777, 0o640),
            ("u=rw+x", 0o000, 0o700),
            ("u=rwx-rx", 0o777, 0o277),
            ("u+rwx=rx", 0o000, 0o500),
        ]);
    }

    #[test]
    fn copy_permissions_use_the_current_mode() {
        assert_modes(&[
            ("g=u", 0o750, 0o770),
            ("u=g", 0o750, 0o550),
            ("go=u", 0o700, 0o777),
            ("ug=o", 0o705, 0o555),
            ("o=ug", 0o640, 0o646),
            ("g=urx", 0o640, 0o670),
            ("u=g,g=u", 0o750, 0o550),
            ("g=u,u=g", 0o750, 0o770),
            ("g=u-w", 0o600, 0o640),
            ("g+u", 0o741, 0o771),
            ("g-u", 0o777, 0o707),
        ]);
    }

    #[test]
    fn malformed_modes_are_rejected() {
        for input in [
            "",
            "77",
            "7777",
            "999",
            "abc",
            "u?rw",
            "u@rw",
            "z=rw",
            "u=rwa",
            "u=rwx,,g=rx",
            "u=rwx,g=rx,",
            ",u=rwx,g=rx",
        ] {
            assert!(
                Mode::from_str(input).is_err(),
                "{input:?} should be rejected"
            );
        }
    }
}
