/// Override values for ownership during permission operations.
/// During creation: Override values stored in archive (None = use filesystem)
/// During extraction: Override values restored from archive (None = use archive)
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct OwnerOptions {
    pub(crate) uname: Option<String>,
    pub(crate) gname: Option<String>,
    pub(crate) uid: Option<u32>,
    pub(crate) gid: Option<u32>,
}

/// Unix permission mask representing bits to clear from file mode.
///
/// This newtype ensures the mask value is always valid (0-0o777).
/// Values outside this range are silently masked on construction.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct Umask(u16);

impl Umask {
    /// Creates a new Umask, masking to valid permission bits (0o777).
    #[inline]
    pub(crate) fn new(mask: u16) -> Self {
        Self(mask & 0o777)
    }

    /// Returns the raw mask value (guaranteed to be 0-0o777).
    #[cfg(test)]
    fn bits(self) -> u16 {
        self.0
    }

    /// Applies this umask to the given mode, also clearing special bits (suid/sgid/sticky).
    ///
    /// This implements the standard Unix behavior for non-privileged extraction:
    /// 1. Clear special bits (setuid, setgid, sticky) for security
    /// 2. Apply umask to remove specified permission bits
    #[inline]
    pub(crate) fn apply(self, mode: u16) -> u16 {
        (mode & !0o7000) & !self.0
    }
}

impl OwnerOptions {
    #[inline]
    pub(crate) const fn has_overrides(&self) -> bool {
        self.uname.is_some() || self.gname.is_some() || self.uid.is_some() || self.gid.is_some()
    }
}

/// How to handle file mode bits (permissions).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum ModeStrategy {
    /// Don't preserve mode bits
    #[default]
    Never,
    /// Preserve mode bits from source
    Preserve,
    /// Restore mode bits with mask applied and suid/sgid/sticky cleared.
    ///
    /// The inner `Umask` value specifies which permission bits to remove
    /// from the archived mode. This is the same semantics as the Unix umask.
    Masked(Umask),
}

/// How to handle file ownership (uid/gid/uname/gname).
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) enum OwnerStrategy {
    /// Don't restore ownership
    #[default]
    Never,
    /// Restore ownership with optional overrides
    Preserve { options: OwnerOptions },
}

/// Maps a CLI-produced owner/group name to an optional owner-facet name:
/// empty → `None` (no `fONm`/`fGNm`, e.g. `--numeric-owner`); otherwise the
/// largest UTF-8-char-boundary prefix ≤255 bytes (the owner-facet wire bound).
pub(crate) fn owner_name_opt(s: &str) -> Option<pna::OwnerUserName> {
    owner_name_bounded(s)
        .map(|t| pna::OwnerUserName::new(t).expect("owner_name_bounded guarantees <= 255 bytes"))
}

pub(crate) fn owner_group_name_opt(s: &str) -> Option<pna::OwnerGroupName> {
    owner_name_bounded(s)
        .map(|t| pna::OwnerGroupName::new(t).expect("owner_name_bounded guarantees <= 255 bytes"))
}

fn owner_name_bounded(s: &str) -> Option<&str> {
    if s.is_empty() {
        return None;
    }
    const MAX: usize = u8::MAX as usize;
    if s.len() <= MAX {
        return Some(s);
    }
    let mut end = MAX;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    Some(&s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn umask_new_masks_to_valid_range() {
        assert_eq!(Umask::new(0o777).bits(), 0o777);
        assert_eq!(Umask::new(0o022).bits(), 0o022);
        assert_eq!(Umask::new(0o000).bits(), 0o000);
        // Values above 0o777 are masked
        assert_eq!(Umask::new(0o1000).bits(), 0o000);
        assert_eq!(Umask::new(0o7777).bits(), 0o777);
        assert_eq!(Umask::new(0o4022).bits(), 0o022);
    }

    #[test]
    fn umask_apply_clears_suid_bit() {
        let umask = Umask::new(0o000);
        assert_eq!(umask.apply(0o4755), 0o755);
    }

    #[test]
    fn umask_apply_clears_sgid_bit() {
        let umask = Umask::new(0o000);
        assert_eq!(umask.apply(0o2755), 0o755);
    }

    #[test]
    fn umask_apply_clears_sticky_bit() {
        let umask = Umask::new(0o000);
        assert_eq!(umask.apply(0o1755), 0o755);
    }

    #[test]
    fn umask_apply_clears_all_special_bits() {
        let umask = Umask::new(0o000);
        assert_eq!(umask.apply(0o7777), 0o777);
        assert_eq!(umask.apply(0o7755), 0o755);
    }

    #[test]
    fn umask_apply_masks_permission_bits() {
        // umask 0o022 removes group/other write bits
        let umask = Umask::new(0o022);
        assert_eq!(umask.apply(0o777), 0o755);
        assert_eq!(umask.apply(0o666), 0o644);

        // umask 0o077 removes all group/other bits
        let umask = Umask::new(0o077);
        assert_eq!(umask.apply(0o755), 0o700);
        assert_eq!(umask.apply(0o777), 0o700);

        // umask 0o027 removes group write and all other bits
        let umask = Umask::new(0o027);
        assert_eq!(umask.apply(0o755), 0o750);
    }

    #[test]
    fn umask_apply_combined_special_bits_and_mask() {
        // Both special bits clearing and umask application
        let umask = Umask::new(0o027);
        assert_eq!(umask.apply(0o4755), 0o750);
        assert_eq!(umask.apply(0o7777), 0o750);
    }

    #[test]
    fn owner_name_opt_empty_is_none() {
        assert!(owner_name_opt("").is_none());
        assert!(owner_group_name_opt("").is_none());
    }

    #[test]
    fn owner_name_opt_passes_through_short() {
        assert_eq!(owner_name_opt("alice").unwrap().as_str(), "alice");
        let exactly_255 = "a".repeat(255);
        assert_eq!(owner_name_opt(&exactly_255).unwrap().as_str(), exactly_255);
    }

    #[test]
    fn owner_name_opt_truncates_long_ascii_to_255() {
        assert_eq!(
            owner_name_opt(&"a".repeat(300)).unwrap().as_str().len(),
            255
        );
        assert_eq!(
            owner_name_opt(&"a".repeat(256)).unwrap().as_str().len(),
            255
        );
    }

    #[test]
    fn owner_name_opt_truncates_on_utf8_boundary() {
        let two_byte_char = 'é';
        assert_eq!(two_byte_char.len_utf8(), 2);
        let s = String::from(two_byte_char).repeat(200); // 400 bytes
        let out = owner_name_opt(&s).unwrap();
        assert_eq!(out.as_str().len(), 254);
        assert_eq!(out.as_str().chars().count(), 127);
        assert!(out.as_str().chars().all(|c| c == two_byte_char));
    }
}
