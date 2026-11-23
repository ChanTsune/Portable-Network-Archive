//! Draft aCLS access-control metadata encoding.
//!
//! The envelope identifies a model and an immutable wire representation.
//! It does not resolve principals or apply ACLs to a filesystem.
use std::io::{self, ErrorKind};

use crate::{Chunk, ChunkType, RawChunk};

/// POSIX.1e ACL, draft version 1.
pub const POSIX1E: u16 = 1;
/// PNA-extended NFSv4 ACL, draft version 1.
pub const NFS4: u16 = 2;
/// Windows NT self-relative security descriptor, draft version 1.
pub const WINDOWS_NT: u16 = 3;
/// Darwin ACL, draft version 1.
pub const DARWIN: u16 = 4;
/// PNA-extended NFSv4.1 DACL/SACL, draft version 1.
pub const NFS41: u16 = 5;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, message)
}

/// An aCLS body, including the two-byte format identifier.
///
/// Unknown nonzero identifiers are preserved without interpretation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AclChunk {
    format_id: u16,
    payload: Vec<u8>,
}

impl AclChunk {
    /// Constructs the envelope from a format identifier and its payload.
    ///
    /// The payload is not validated as a supported ACL model by this method.
    ///
    /// # Errors
    /// Returns an error if the identifier is zero.
    #[inline]
    pub fn new(format_id: u16, payload: impl Into<Vec<u8>>) -> io::Result<Self> {
        if format_id == 0 {
            return Err(invalid("reserved ACL format ID"));
        }
        Ok(Self {
            format_id,
            payload: payload.into(),
        })
    }

    /// Parses the enclosing chunk's data.
    ///
    /// # Errors
    /// Returns an error for a missing or reserved format identifier.
    #[inline]
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        let id = data
            .get(..2)
            .ok_or_else(|| invalid("truncated ACL format ID"))?;
        Self::new(u16::from_be_bytes([id[0], id[1]]), &data[2..])
    }

    /// Returns the ACL format identifier.
    #[inline]
    pub const fn format_id(&self) -> u16 {
        self.format_id
    }

    /// Returns the model-specific payload bytes.
    #[inline]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Encodes the chunk data.
    #[inline]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(2 + self.payload.len());
        result.extend_from_slice(&self.format_id.to_be_bytes());
        result.extend_from_slice(&self.payload);
        result
    }

    /// Produces the public, ancillary, unsafe-to-copy aCLS chunk.
    #[inline]
    pub fn into_raw_chunk(self) -> RawChunk {
        RawChunk::from_data(ChunkType::aCLS, self.to_bytes())
    }

    /// Checks the supported draft model representation.
    ///
    /// # Errors
    /// Returns an error for truncated or structurally invalid known formats.
    ///
    /// Unknown identifiers are opaque and are not treated as malformed.
    #[inline]
    pub fn validate(&self) -> io::Result<()> {
        match self.format_id {
            POSIX1E => {
                PosixAcl::decode(&self.payload)?;
            }
            NFS4 => validate_nfs4(&self.payload)?,
            NFS41 => validate_nfs41(&self.payload)?,
            WINDOWS_NT => validate_windows(&self.payload)?,
            DARWIN => validate_darwin(&self.payload)?,
            _ => {}
        }
        Ok(())
    }
}

/// POSIX ACL entry tag values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PosixTag {
    /// The file owner.
    UserObj = 0,
    /// A named user.
    User = 1,
    /// The owning group.
    GroupObj = 2,
    /// A named group.
    Group = 3,
    /// The group-class mask.
    Mask = 4,
    /// Everyone else.
    Other = 5,
}

impl TryFrom<u8> for PosixTag {
    type Error = io::Error;
    #[inline]
    fn try_from(value: u8) -> io::Result<Self> {
        match value {
            0 => Ok(Self::UserObj),
            1 => Ok(Self::User),
            2 => Ok(Self::GroupObj),
            3 => Ok(Self::Group),
            4 => Ok(Self::Mask),
            5 => Ok(Self::Other),
            _ => Err(invalid("unknown POSIX ACL tag")),
        }
    }
}

/// One POSIX.1e access or default ACL entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PosixAce {
    /// ACL tag.
    pub tag: PosixTag,
    /// Permission bits: read=4, write=2, execute=1.
    pub permissions: u8,
    /// UID or GID for named principals; absent for all other tags.
    pub qualifier: Option<u64>,
    /// Optional UTF-8 name for named principals.
    pub name: String,
}

/// Complete access ACL and optional default ACL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PosixAcl {
    /// Access entries; at least USER_OBJ, GROUP_OBJ, OTHER.
    pub access: Vec<PosixAce>,
    /// Inheritable directory defaults; empty if not recorded as present.
    pub default: Vec<PosixAce>,
}

impl PosixAcl {
    /// Decodes a complete POSIX.1e payload, rejecting structural violations.
    ///
    /// # Errors
    /// Returns an error for invalid entries, counts, or UTF-8.
    #[inline]
    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        let mut reader = Reader::new(bytes);
        let access = read_posix_entries(&mut reader)?;
        let default = read_posix_entries(&mut reader)?;
        if !reader.rest.is_empty() {
            return Err(invalid("trailing POSIX ACL data"));
        }
        validate_posix_list(&access, false)?;
        validate_posix_list(&default, true)?;
        Ok(Self { access, default })
    }

    /// Encodes a validated POSIX.1e payload.
    ///
    /// # Errors
    /// Returns an error for invalid entries or oversized names.
    #[inline]
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        validate_posix_list(&self.access, false)?;
        validate_posix_list(&self.default, true)?;
        let mut out = Vec::new();
        for entries in [&self.access, &self.default] {
            let count =
                u32::try_from(entries.len()).map_err(|_| invalid("too many POSIX ACL entries"))?;
            out.extend_from_slice(&count.to_be_bytes());
            for ace in entries {
                out.push(ace.tag as u8);
                out.push(ace.permissions);
                if let Some(id) = ace.qualifier {
                    out.extend_from_slice(&id.to_be_bytes());
                    let name = ace.name.as_bytes();
                    let len = u16::try_from(name.len())
                        .map_err(|_| invalid("POSIX principal name too long"))?;
                    out.extend_from_slice(&len.to_be_bytes());
                    out.extend_from_slice(name);
                }
            }
        }
        Ok(out)
    }

    /// Encodes a public aCLS chunk carrying this POSIX ACL.
    ///
    /// # Errors
    /// Returns an error for an invalid POSIX ACL.
    #[inline]
    pub fn into_acl_chunk(self) -> io::Result<AclChunk> {
        AclChunk::new(POSIX1E, self.encode()?)
    }
}

fn read_posix_entries(reader: &mut Reader<'_>) -> io::Result<Vec<PosixAce>> {
    let count = reader.u32()? as usize;
    if count > reader.rest.len() / 2 {
        return Err(invalid("POSIX ACL entry count exceeds remaining bytes"));
    }
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let tag = PosixTag::try_from(reader.u8()?)?;
        let permissions = reader.u8()?;
        let (qualifier, name) = match tag {
            PosixTag::User | PosixTag::Group => {
                let id = reader.u64()?;
                let len = reader.u16()? as usize;
                let name = std::str::from_utf8(reader.take(len)?)
                    .map_err(|_| invalid("POSIX principal name is not UTF-8"))?;
                (Some(id), name.to_owned())
            }
            _ => (None, String::new()),
        };
        entries.push(PosixAce {
            tag,
            permissions,
            qualifier,
            name,
        });
    }
    Ok(entries)
}

fn validate_posix_list(entries: &[PosixAce], default: bool) -> io::Result<()> {
    if default && entries.is_empty() {
        return Ok(());
    }
    if entries.len() < 3 {
        return Err(invalid("incomplete POSIX access/default ACL"));
    }
    let mut seen = [false; 6];
    let mut users = std::collections::HashSet::new();
    let mut groups = std::collections::HashSet::new();
    let mut named = false;
    for ace in entries {
        if ace.permissions & !7 != 0 {
            return Err(invalid("invalid POSIX ACL permission bits"));
        }
        let tag = ace.tag as usize;
        match ace.tag {
            PosixTag::User | PosixTag::Group => {
                named = true;
                let id = ace
                    .qualifier
                    .ok_or_else(|| invalid("missing POSIX qualifier"))?;
                let ids = if ace.tag == PosixTag::User {
                    &mut users
                } else {
                    &mut groups
                };
                if !ids.insert(id) {
                    return Err(invalid("duplicate POSIX named principal"));
                }
                if ace.name.len() > u16::MAX as usize {
                    return Err(invalid("POSIX principal name too long"));
                }
            }
            _ => {
                if seen[tag] || ace.qualifier.is_some() || !ace.name.is_empty() {
                    return Err(invalid("invalid POSIX singleton ACL entry"));
                }
                seen[tag] = true;
            }
        }
    }
    if !(seen[0] && seen[2] && seen[5]) || (named && !seen[4]) {
        return Err(invalid("missing required POSIX ACL entry or mask"));
    }
    Ok(())
}

struct Reader<'a> {
    rest: &'a [u8],
}
impl<'a> Reader<'a> {
    fn new(rest: &'a [u8]) -> Self {
        Self { rest }
    }
    fn take(&mut self, len: usize) -> io::Result<&'a [u8]> {
        let bytes = self
            .rest
            .get(..len)
            .ok_or_else(|| invalid("truncated ACL payload"))?;
        self.rest = &self.rest[len..];
        Ok(bytes)
    }
    fn u8(&mut self) -> io::Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_be_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_be_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_be_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn utf8_xdr(&mut self) -> io::Result<()> {
        let len = self.u32()? as usize;
        std::str::from_utf8(self.take(len)?).map_err(|_| invalid("invalid NFS ACL UTF-8"))?;
        let pad = (4 - len % 4) % 4;
        if self.take(pad)?.iter().any(|b| *b != 0) {
            return Err(invalid("nonzero XDR padding"));
        }
        Ok(())
    }
}

fn read_nfs_aces(reader: &mut Reader<'_>, expected: Option<bool>) -> io::Result<()> {
    let count = reader.u32()? as usize;
    if count > reader.rest.len() / 20 {
        return Err(invalid("NFS ACL entry count exceeds remaining bytes"));
    }
    for _ in 0..count {
        let kind = reader.u32()?;
        if let Some(is_sacl) = expected {
            if (kind > 1) != is_sacl || kind > 3 {
                return Err(invalid("ACE in wrong NFSv4.1 ACL component"));
            }
        }
        let _flags = reader.u32()?;
        let _mask = reader.u32()?;
        match reader.u32()? {
            0 => reader.utf8_xdr()?,
            1 => {
                let has_numeric = match reader.u32()? {
                    0 => false,
                    1 => {
                        reader.u64()?;
                        true
                    }
                    _ => return Err(invalid("invalid XDR optional principal discriminator")),
                };
                let len = reader.u32()? as usize;
                if len == 0 && !has_numeric {
                    // A numeric qualifier is required when the name is absent.
                    // Rewind is unnecessary: inspect the encoded optional boolean above.
                    // This combination is structurally valid only when numeric ID exists.
                    return Err(invalid("empty NFS principal name needs numeric ID"));
                }
                std::str::from_utf8(reader.take(len)?)
                    .map_err(|_| invalid("invalid NFS ACL principal UTF-8"))?;
                let pad = (4 - len % 4) % 4;
                if reader.take(pad)?.iter().any(|b| *b != 0) {
                    return Err(invalid("nonzero XDR padding"));
                }
            }
            _ => return Err(invalid("unknown NFS principal kind")),
        }
    }
    Ok(())
}
fn validate_nfs4(data: &[u8]) -> io::Result<()> {
    let mut reader = Reader::new(data);
    read_nfs_aces(&mut reader, None)?;
    if !reader.rest.is_empty() {
        return Err(invalid("trailing NFSv4 ACL data"));
    }
    Ok(())
}
fn validate_nfs41(data: &[u8]) -> io::Result<()> {
    let mut reader = Reader::new(data);
    let presence = reader.u32()?;
    if presence == 0 || presence & !3 != 0 {
        return Err(invalid("invalid NFSv4.1 DACL/SACL presence"));
    }
    for (bit, is_sacl) in [(1, false), (2, true)] {
        if presence & bit != 0 {
            let _flags = reader.u32()?;
            read_nfs_aces(&mut reader, Some(is_sacl))?;
        }
    }
    if !reader.rest.is_empty() {
        return Err(invalid("trailing NFSv4.1 ACL data"));
    }
    Ok(())
}
fn validate_windows(data: &[u8]) -> io::Result<()> {
    let (&coverage, descriptor) = data
        .split_first()
        .ok_or_else(|| invalid("missing Windows ACL coverage"))?;
    if coverage == 0 || coverage & !3 != 0 || descriptor.len() < 20 {
        return Err(invalid("invalid Windows ACL envelope"));
    }
    let control = u16::from_le_bytes([descriptor[2], descriptor[3]]);
    if descriptor[0] != 1 || control & 0x8000 == 0 {
        return Err(invalid("invalid self-relative security descriptor"));
    }
    for start in [4, 8] {
        if descriptor[start..start + 4] != [0; 4] {
            return Err(invalid("Windows owner/group must be separate"));
        }
    }
    for (start, present) in [(12, 0x0010u16), (16, 0x0004u16)] {
        let offset = u32::from_le_bytes(descriptor[start..start + 4].try_into().unwrap()) as usize;
        if offset != 0 {
            if control & present == 0 {
                return Err(invalid("Windows ACL offset without PRESENT control bit"));
            }
            let header = descriptor
                .get(offset..offset.saturating_add(8))
                .ok_or_else(|| invalid("Windows ACL offset out of range"))?;
            let acl_size = u16::from_le_bytes([header[2], header[3]]) as usize;
            if acl_size < 8
                || descriptor
                    .get(offset..offset.saturating_add(acl_size))
                    .is_none()
            {
                return Err(invalid("Windows ACL size out of range"));
            }
        }
    }
    Ok(())
}
fn validate_darwin(data: &[u8]) -> io::Result<()> {
    let mut reader = Reader::new(data);
    let count = reader.u32()?;
    let _flags = reader.u32()?;
    if count == u32::MAX {
        return if reader.rest.is_empty() {
            Ok(())
        } else {
            Err(invalid("trailing Darwin no-ACL data"))
        };
    }
    if count as usize > reader.rest.len() / 28 {
        return Err(invalid("Darwin ACL entry count exceeds remaining bytes"));
    }
    for _ in 0..count {
        reader.take(16)?;
        reader.u32()?;
        reader.u32()?;
        let kind = reader.u8()?;
        let present = reader.u8()?;
        if kind > 2 || present > 1 {
            return Err(invalid(
                "invalid Darwin ACL principal kind or numeric presence",
            ));
        }
        if present == 1 {
            reader.u64()?;
        }
        let len = reader.u16()? as usize;
        std::str::from_utf8(reader.take(len)?).map_err(|_| invalid("invalid Darwin ACL name"))?;
    }
    if !reader.rest.is_empty() {
        return Err(invalid("trailing Darwin ACL data"));
    }
    Ok(())
}

impl<T> crate::NormalEntry<T>
where
    RawChunk<T>: crate::Chunk,
{
    /// Returns the draft ACL of an entry, if present.
    ///
    /// Unknown formats are kept as opaque payloads.
    ///
    /// # Errors
    /// Returns an error for multiple ACL chunks or a malformed known payload.
    #[inline]
    pub fn acl_chunk(&self) -> io::Result<Option<AclChunk>> {
        let mut found = None;
        for chunk in self.extra_chunks() {
            if chunk.ty() == ChunkType::aCLS {
                if found.is_some() {
                    return Err(invalid("duplicate aCLS chunks"));
                }
                let acl = AclChunk::from_bytes(chunk.data())?;
                acl.validate()?;
                found = Some(acl);
            }
        }
        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn base_acl() -> PosixAcl {
        PosixAcl {
            access: vec![
                PosixAce {
                    tag: PosixTag::UserObj,
                    permissions: 7,
                    qualifier: None,
                    name: String::new(),
                },
                PosixAce {
                    tag: PosixTag::GroupObj,
                    permissions: 5,
                    qualifier: None,
                    name: String::new(),
                },
                PosixAce {
                    tag: PosixTag::Other,
                    permissions: 0,
                    qualifier: None,
                    name: String::new(),
                },
            ],
            default: vec![],
        }
    }
    #[test]
    fn posix_roundtrip_named_default_and_utf8() {
        let mut a = base_acl();
        a.access.insert(
            1,
            PosixAce {
                tag: PosixTag::User,
                permissions: 5,
                qualifier: Some(1001),
                name: "ユーザー".into(),
            },
        );
        a.access.push(PosixAce {
            tag: PosixTag::Mask,
            permissions: 5,
            qualifier: None,
            name: String::new(),
        });
        a.default = base_acl().access;
        let data = a.clone().into_acl_chunk().unwrap();
        assert_eq!(data.format_id(), POSIX1E);
        data.validate().unwrap();
        assert_eq!(PosixAcl::decode(data.payload()).unwrap(), a);
        let raw = data.clone().into_raw_chunk();
        assert_eq!(raw.ty(), ChunkType::aCLS);
        assert_eq!(AclChunk::from_bytes(raw.data()).unwrap(), data);
    }
    #[test]
    fn corrupt_posix_and_envelope_rejected() {
        assert!(AclChunk::from_bytes(&[0]).is_err());
        assert!(AclChunk::new(0, vec![]).is_err());
        let mut x = base_acl().encode().unwrap();
        x[5] = 0xff;
        assert!(PosixAcl::decode(&x).is_err());
        let mut x = base_acl().encode().unwrap();
        x.push(0);
        assert!(PosixAcl::decode(&x).is_err());
    }
    #[test]
    fn nfs4_empty_and_unknown_private_roundtrip() {
        AclChunk::new(NFS4, 0u32.to_be_bytes())
            .unwrap()
            .validate()
            .unwrap();
        let b = AclChunk::new(0x8001, vec![0xff, 0x00]).unwrap();
        b.validate().unwrap();
        assert_eq!(AclChunk::from_bytes(&b.to_bytes()).unwrap(), b);
    }
    #[test]
    fn nfs41_empty_components_roundtrip() {
        let mut bytes = vec![];
        for value in [3u32, 2, 0, 4, 0] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        AclChunk::new(NFS41, bytes).unwrap().validate().unwrap();
    }
    #[test]
    fn archive_entry_roundtrip_all_draft_formats() {
        use crate::{Archive, FileEntryBuilder};
        let posix = base_acl().encode().unwrap();
        let nfs4 = 0u32.to_be_bytes().to_vec();
        let mut nfs41 = Vec::new();
        for n in [3u32, 2, 0, 4, 0] {
            nfs41.extend_from_slice(&n.to_be_bytes());
        }
        let mut sd = vec![0u8; 20];
        sd[0] = 1;
        sd[2..4].copy_from_slice(&0x8000u16.to_le_bytes());
        let mut windows = vec![1];
        windows.extend_from_slice(&sd);
        let mut darwin = vec![];
        darwin.extend_from_slice(&u32::MAX.to_be_bytes());
        darwin.extend_from_slice(&0u32.to_be_bytes());
        for (id, payload) in [
            (POSIX1E, posix),
            (NFS4, nfs4),
            (NFS41, nfs41),
            (WINDOWS_NT, windows),
            (DARWIN, darwin),
            (0x8001, vec![0xfe, 0xff]),
        ] {
            let acl = AclChunk::new(id, payload).unwrap();
            acl.validate().unwrap();
            let mut serialized = Vec::new();
            {
                let mut archive = Archive::write_header(&mut serialized).unwrap();
                let mut entry = FileEntryBuilder::new("file.txt".into()).unwrap();
                entry.add_extra_chunk(acl.clone().into_raw_chunk());
                use std::io::Write;
                entry.write_all(b"ACL payload roundtrip").unwrap();
                archive.add_entry(entry.build().unwrap()).unwrap();
                archive.finalize().unwrap();
            }
            let mut archive = Archive::read_header(&serialized[..]).unwrap();
            let entry = archive.entries().skip_solid().next().unwrap().unwrap();
            let actual = entry.acl_chunk().unwrap().unwrap();
            assert_eq!(actual, acl);
        }
    }

    #[test]
    fn duplicate_acl_chunk_is_invalid_metadata() {
        use crate::DirEntryBuilder;
        let entry = DirEntryBuilder::new("dir".into()).build().unwrap();
        let chunk = AclChunk::new(0x8001, vec![42]).unwrap().into_raw_chunk();
        let entry = entry.with_extra_chunks(vec![chunk.clone(), chunk]);
        assert!(entry.acl_chunk().is_err());
    }

    #[test]
    fn malformed_acl_metadata_does_not_hide_valid_file_contents() {
        use crate::{Archive, FileEntryBuilder, ReadOptions};
        use std::io::{Read, Write};

        // aCLS is ancillary. A broken format ID or payload must be reported
        // as a metadata error without silently discarding valid file data.
        for malformed in [
            vec![0],       // truncated format ID
            vec![0, 0],    // reserved format ID
            vec![0, 1, 0], // truncated POSIX.1e ACL
        ] {
            let mut serialized = Vec::new();
            {
                let mut archive = Archive::write_header(&mut serialized).unwrap();
                let mut builder = FileEntryBuilder::new("data.txt".into()).unwrap();
                builder.add_extra_chunk(RawChunk::from_data(ChunkType::aCLS, malformed));
                builder.write_all(b"preserved file contents").unwrap();
                archive.add_entry(builder.build().unwrap()).unwrap();
                archive.finalize().unwrap();
            }
            let mut archive = Archive::read_header(&serialized[..]).unwrap();
            let entry = archive.entries().skip_solid().next().unwrap().unwrap();
            assert!(entry.acl_chunk().is_err());
            let mut content = Vec::new();
            entry
                .reader(ReadOptions::builder().build())
                .unwrap()
                .read_to_end(&mut content)
                .unwrap();
            assert_eq!(content, b"preserved file contents");
        }
    }

    #[test]
    fn independent_archive_fixture_decodes_all_formats() {
        use crate::{Archive, ReadOptions};
        use std::io::Read;
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../resources/test/acl_draft_probe.pna"
        ));
        let mut archive = Archive::read_header(&bytes[..]).unwrap();
        let mut entries = archive.entries().skip_solid();
        for (name, format) in [
            ("posix.txt", POSIX1E),
            ("nfs4.txt", NFS4),
            ("windows.txt", WINDOWS_NT),
            ("darwin.txt", DARWIN),
            ("nfsv41.txt", NFS41),
            ("unknown.txt", 0x8001),
        ] {
            let entry = entries.next().unwrap().unwrap();
            assert_eq!(entry.name().as_str(), name);
            let acl = entry.acl_chunk().unwrap().unwrap();
            assert_eq!(acl.format_id(), format);
            acl.validate().unwrap();
            let mut result = String::new();
            entry
                .reader(ReadOptions::builder().build())
                .unwrap()
                .read_to_string(&mut result)
                .unwrap();
            assert_eq!(result, format!("content for {name}"));
        }
        assert!(entries.next().is_none());
    }

    #[test]
    fn windows_acl_presence_distinguishes_absent_null_and_empty() {
        let mut sd = vec![0u8; 28];
        sd[0] = 1;
        sd[16..20].copy_from_slice(&20u32.to_le_bytes());
        sd[20..28].copy_from_slice(&[2, 0, 8, 0, 0, 0, 0, 0]);
        let validate = |sd: &[u8]| {
            let mut payload = vec![1u8];
            payload.extend_from_slice(sd);
            AclChunk::new(WINDOWS_NT, payload)
                .unwrap()
                .validate()
        };

        // An offset without the corresponding PRESENT bit is inconsistent.
        sd[2..4].copy_from_slice(&0x8000u16.to_le_bytes());
        assert!(validate(&sd).is_err());
        // PRESENT with a nonzero offset means an explicit empty ACL.
        sd[2..4].copy_from_slice(&0x8004u16.to_le_bytes());
        validate(&sd).unwrap();
        // PRESENT with offset zero means a NULL ACL.
        sd[16..20].fill(0);
        validate(&sd).unwrap();
        // Neither PRESENT nor offset means an absent ACL, not a NULL ACL.
        sd[2..4].copy_from_slice(&0x8000u16.to_le_bytes());
        validate(&sd).unwrap();
    }

    #[test]
    fn malformed_known_formats_and_unknown_ids() {
        for (id, payload) in [
            (POSIX1E, vec![0u8; 8]),
            (NFS4, 1u32.to_be_bytes().to_vec()),
            (NFS41, 0u32.to_be_bytes().to_vec()),
            (WINDOWS_NT, vec![1u8; 21]),
            (DARWIN, 1u32.to_be_bytes().to_vec()),
        ] {
            assert!(
                AclChunk::new(id, payload).unwrap().validate().is_err(),
                "malformed public format {id} must be rejected"
            );
        }
        for id in [0x0006, 0x8000, 0xffff] {
            let unknown = AclChunk::new(id, vec![0xff, 0, 0xfe]).unwrap();
            unknown.validate().unwrap();
            assert_eq!(AclChunk::from_bytes(&unknown.to_bytes()).unwrap(), unknown);
        }
    }

    #[test]
    fn windows_and_darwin_envelopes() {
        let mut sd = vec![0u8; 20];
        sd[0] = 1;
        sd[2..4].copy_from_slice(&0x8000u16.to_le_bytes());
        let mut payload = vec![1];
        payload.extend_from_slice(&sd);
        AclChunk::new(WINDOWS_NT, payload)
            .unwrap()
            .validate()
            .unwrap();
        let mut payload = vec![];
        payload.extend_from_slice(&u32::MAX.to_be_bytes());
        payload.extend_from_slice(&0u32.to_be_bytes());
        AclChunk::new(DARWIN, payload).unwrap().validate().unwrap();
    }
}
