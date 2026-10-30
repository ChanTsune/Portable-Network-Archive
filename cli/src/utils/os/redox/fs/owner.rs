use nix::unistd::{Gid, Uid};
use std::{fs, io};

fn split_columns(line: &str) -> std::str::Split<'_, char> {
    if line.contains(';') {
        line.split(';')
    } else {
        line.split(':')
    }
}

pub(crate) struct User {
    uid: Uid,
    name: String,
    gid: Gid,
}

impl User {
    #[inline]
    pub(crate) fn from_uid(uid: Uid) -> io::Result<Self> {
        let passwd = fs::read_to_string("/etc/passwd")?;
        Self::lookup(&passwd, |user| user.uid == uid)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("uid {uid} not found")))
    }

    #[inline]
    pub(crate) fn from_name(name: &str) -> io::Result<Self> {
        let passwd = fs::read_to_string("/etc/passwd")?;
        Self::lookup(&passwd, |user| user.name == name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("username {name} not found"),
            )
        })
    }

    fn lookup(passwd: &str, predicate: impl Fn(&Self) -> bool) -> Option<Self> {
        passwd.lines().filter_map(Self::parse).find(predicate)
    }

    fn parse(line: &str) -> Option<Self> {
        let mut columns = split_columns(line);
        let name = columns.next()?;
        let uid = columns.next()?.parse().ok()?;
        let gid = columns.next()?.parse().ok()?;
        Some(Self {
            uid: Uid::from_raw(uid),
            name: name.to_string(),
            gid: Gid::from_raw(gid),
        })
    }

    #[inline]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub(crate) fn as_raw(&self) -> u32 {
        self.uid.as_raw()
    }

    #[inline]
    pub(crate) fn primary_gid(&self) -> Option<u32> {
        Some(self.gid.as_raw())
    }
}

pub(crate) struct Group {
    gid: Gid,
    name: String,
}

impl Group {
    #[inline]
    pub(crate) fn from_gid(gid: Gid) -> io::Result<Self> {
        let group = fs::read_to_string("/etc/group")?;
        Self::lookup(&group, |group| group.gid == gid)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("gid {gid} not found")))
    }

    #[inline]
    pub(crate) fn from_name(name: &str) -> io::Result<Self> {
        let group = fs::read_to_string("/etc/group")?;
        Self::lookup(&group, |group| group.name == name).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, format!("gname {name} not found"))
        })
    }

    fn lookup(group: &str, predicate: impl Fn(&Self) -> bool) -> Option<Self> {
        group.lines().filter_map(Self::parse).find(predicate)
    }

    fn parse(line: &str) -> Option<Self> {
        let mut columns = split_columns(line);
        let name = columns.next()?;
        let _password = columns.next()?;
        let gid = columns.next()?.parse().ok()?;
        Some(Self {
            gid: Gid::from_raw(gid),
            name: name.to_string(),
        })
    }

    #[inline]
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub(crate) fn as_raw(&self) -> u32 {
        self.gid.as_raw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSWD: &str = "\
root;0;0;root;file:/root;file:/bin/ion
user;1000;1001;user;file:/home/user;file:/bin/ion
";
    const GROUP: &str = "\
root;x;0;root
wheel;x;1;user,root
user;x;1000;user
";

    #[test]
    fn user_lookup_by_name() {
        let user = User::lookup(PASSWD, |user| user.name() == "user").unwrap();
        assert_eq!(user.as_raw(), 1000);
        assert_eq!(user.name(), "user");
        assert_eq!(user.primary_gid(), Some(1001));
    }

    #[test]
    fn user_lookup_by_uid() {
        let user = User::lookup(PASSWD, |user| user.as_raw() == 0).unwrap();
        assert_eq!(user.name(), "root");
        assert_eq!(user.primary_gid(), Some(0));
    }

    #[test]
    fn user_lookup_colon_delimited() {
        let user = User::lookup("root:0:0:root:file:/root:file:/bin/ion\n", |user| {
            user.name() == "root"
        })
        .unwrap();
        assert_eq!(user.as_raw(), 0);
        assert_eq!(user.primary_gid(), Some(0));
    }

    #[test]
    fn user_lookup_missing() {
        assert!(User::lookup(PASSWD, |user| user.name() == "missing").is_none());
    }

    #[test]
    fn group_lookup_by_name() {
        let group = Group::lookup(GROUP, |group| group.name() == "wheel").unwrap();
        assert_eq!(group.as_raw(), 1);
        assert_eq!(group.name(), "wheel");
    }

    #[test]
    fn group_lookup_by_gid() {
        let group = Group::lookup(GROUP, |group| group.as_raw() == 1000).unwrap();
        assert_eq!(group.name(), "user");
    }

    #[test]
    fn group_lookup_colon_delimited() {
        let group =
            Group::lookup("wheel:x:1:user,root\n", |group| group.name() == "wheel").unwrap();
        assert_eq!(group.as_raw(), 1);
    }

    #[test]
    fn group_lookup_missing() {
        assert!(Group::lookup(GROUP, |group| group.name() == "missing").is_none());
    }
}
