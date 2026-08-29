//! Key and nonce derivation for cipher mode 2 (GCM STREAM).
//!
//! Binding the entry header into the stream key means an entry cannot be moved
//! or renamed without re-encrypting it. The key confirmation exists so that a
//! decoder can tell a wrong password from tampered data before it processes a
//! segment, which a GCM tag alone cannot distinguish.

use crate::{ChunkType, error::AeadError};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use std::fmt;
use std::num::NonZeroU32;
use subtle::ConstantTimeEq;

pub(crate) const STREAM_HEADER_LEN: usize = 75;
pub(crate) const GCM_TAG_LEN: usize = 16;
pub(crate) const MAX_SEGMENT_SIZE: u32 = 67_108_864; // 64 MiB
pub(crate) const DEFAULT_SEGMENT_SIZE: u32 = 1_048_576; // 1 MiB
const DOMAIN_TAG: &[u8; 13] = b"PNA-STREAM-v1";
const KEY_CONFIRMATION_INFO: &[u8; 9] = b"PNA-KC-v1";
const ENTRY_CONTEXT_LEN: usize = 88;

/// The key confirmation value carried by a stream header.
///
/// Deliberately not [`PartialEq`]: one operand of a real comparison is always
/// attacker-supplied archive bytes, and `==` would leak how far a guessed
/// password's confirmation agrees with the stored one. Use
/// [`StreamHeader::confirms_key`], which compares in constant time.
#[derive(Debug, Clone, Copy)]
pub(crate) struct KeyConfirmation([u8; 32]);

impl KeyConfirmation {
    #[cfg(test)]
    #[inline]
    pub(crate) const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

/// A per-stream AEAD key derived from the master key and the entry context.
///
/// Distinct from the master key so that the two cannot be swapped at a cipher
/// construction site, where the mistake would produce archives that decrypt
/// only with the same mistake.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct StreamKey([u8; 32]);

impl StreamKey {
    #[cfg(test)]
    #[inline]
    pub(crate) const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[inline]
    pub(crate) fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for StreamKey {
    /// Redacted: this is key material, and `StreamKey` is reachable from types
    /// that derive [`Debug`].
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StreamKey(..)")
    }
}

/// A segment size that is in range: non-zero and at most [`MAX_SEGMENT_SIZE`].
///
/// [`SegmentSize::new`] is the only way to obtain one, so a value of this type
/// carries the check with it — a segment loop cannot be handed a zero that would
/// leave it making no progress, and no downstream use has to re-validate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SegmentSize(NonZeroU32);

impl SegmentSize {
    /// Returns [`None`] when out of range, leaving the meaning of that to the
    /// caller: an out-of-range value read off the wire is a malformed
    /// datastream, while one handed in by a caller is invalid input.
    pub(crate) fn new(value: u32) -> Option<Self> {
        match NonZeroU32::new(value) {
            Some(value) if value.get() <= MAX_SEGMENT_SIZE => Some(Self(value)),
            _ => None,
        }
    }

    pub(crate) const fn get(self) -> u32 {
        self.0.get()
    }
}

/// On-wire GCM stream header:
/// `salt(32) || nonce_prefix(7) || segment_size(u32 BE) || key_confirmation(32)`.
#[derive(Debug)]
pub(crate) struct StreamHeader {
    pub(crate) salt: [u8; 32],
    pub(crate) nonce_prefix: [u8; 7],
    segment_size: SegmentSize,
    key_confirmation: KeyConfirmation,
}

impl StreamHeader {
    pub(crate) const fn new(
        salt: [u8; 32],
        nonce_prefix: [u8; 7],
        segment_size: SegmentSize,
        key_confirmation: KeyConfirmation,
    ) -> Self {
        Self {
            salt,
            nonce_prefix,
            segment_size,
            key_confirmation,
        }
    }

    pub(crate) const fn segment_size(&self) -> SegmentSize {
        self.segment_size
    }

    /// Whether the stored key confirmation matches the one `k_master` derives,
    /// compared in constant time.
    pub(crate) fn confirms_key(&self, k_master: &[u8]) -> bool {
        key_confirmation(k_master)
            .0
            .ct_eq(&self.key_confirmation.0)
            .into()
    }

    pub(crate) fn to_bytes(&self) -> [u8; STREAM_HEADER_LEN] {
        let mut bytes = [0u8; STREAM_HEADER_LEN];
        bytes[..32].copy_from_slice(&self.salt);
        bytes[32..39].copy_from_slice(&self.nonce_prefix);
        bytes[39..43].copy_from_slice(&self.segment_size.get().to_be_bytes());
        bytes[43..75].copy_from_slice(&self.key_confirmation.0);
        bytes
    }

    pub(crate) fn try_from_bytes(bytes: &[u8; STREAM_HEADER_LEN]) -> Result<Self, AeadError> {
        let segment_size = SegmentSize::new(u32::from_be_bytes(bytes[39..43].try_into().unwrap()))
            .ok_or(AeadError::Malformed("segment size out of range"))?;
        Ok(Self::new(
            bytes[..32].try_into().unwrap(),
            bytes[32..39].try_into().unwrap(),
            segment_size,
            KeyConfirmation(bytes[43..75].try_into().unwrap()),
        ))
    }
}

pub(crate) fn hkdf_sha256(ikm: &[u8], salt: &[u8], info: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(salt), ikm);
    let mut okm = [0u8; 32];
    hk.expand(info, &mut okm)
        .expect("32 bytes is a valid HKDF-SHA-256 output length");
    okm
}

/// Derives the key confirmation value that every stream header of an archive
/// protected by `k_master` carries.
pub(crate) fn key_confirmation(k_master: &[u8]) -> KeyConfirmation {
    KeyConfirmation(hkdf_sha256(k_master, &[], KEY_CONFIRMATION_INFO))
}

pub(crate) fn entry_context(
    header: &StreamHeader,
    header_chunk_type: ChunkType,
    header_chunk_data: &[u8],
    phsf_chunk_data: &[u8],
) -> [u8; ENTRY_CONTEXT_LEN] {
    let mut ctx = [0u8; ENTRY_CONTEXT_LEN];
    let mut header_hasher = Sha256::new();
    header_hasher.update(header_chunk_type.as_bytes());
    header_hasher.update(header_chunk_data);

    ctx[..13].copy_from_slice(DOMAIN_TAG);
    ctx[13..45].copy_from_slice(&header_hasher.finalize());
    ctx[45..77].copy_from_slice(&Sha256::digest(phsf_chunk_data));
    ctx[77..84].copy_from_slice(&header.nonce_prefix);
    ctx[84..88].copy_from_slice(&header.segment_size.get().to_be_bytes());
    ctx
}

pub(crate) fn derive_stream_key(
    k_master: &[u8],
    header: &StreamHeader,
    header_chunk_type: ChunkType,
    header_chunk_data: &[u8],
    phsf_chunk_data: &[u8],
) -> StreamKey {
    let info = entry_context(
        header,
        header_chunk_type,
        header_chunk_data,
        phsf_chunk_data,
    );
    StreamKey(hkdf_sha256(k_master, &header.salt, &info))
}

pub(crate) fn segment_nonce(nonce_prefix: &[u8; 7], counter: u32, is_final: bool) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[..7].copy_from_slice(nonce_prefix);
    nonce[7..11].copy_from_slice(&counter.to_be_bytes());
    nonce[11] = if is_final { 0x01 } else { 0x00 };
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    const SALT: [u8; 32] = [0x42; 32];
    const PREFIX: [u8; 7] = [0x5A; 7];
    const SEGMENT_SIZE: u32 = 0x01020304;
    const K_MASTER: &[u8] = b"master_key";
    const HEADER_DATA: &[u8] = b"header";
    const PHSF_DATA: &[u8] = b"phsf";

    /// `HKDF-SHA-256(ikm = K_MASTER, salt = SALT, info = entry_context(FHED))`.
    ///
    /// Produced by an RFC 5869 implementation outside this crate. Regenerating it
    /// from `derive_stream_key` would bless whatever that function currently does
    /// and lose the only external check on the derivation.
    const K_STREAM_FHED: StreamKey = StreamKey::from_bytes([
        0xb8, 0x8e, 0x2e, 0xdc, 0x07, 0x53, 0x8b, 0xdd, 0x2b, 0x9a, 0xff, 0xf5, 0x7f, 0xb0, 0xd3,
        0x43, 0x3a, 0x1f, 0x44, 0x98, 0xd2, 0x2a, 0x59, 0x11, 0x50, 0x7e, 0x68, 0x27, 0x59, 0x0f,
        0xad, 0xb5,
    ]);

    /// `HKDF-SHA-256(ikm = "master_key", salt = ∅, info = "PNA-KC-v1")`, produced
    /// by an RFC 5869 implementation outside this crate.
    const K_CONFIRM_MASTER_KEY: [u8; 32] = [
        0xe4, 0x1a, 0x66, 0x1e, 0x64, 0x9b, 0x11, 0x68, 0x18, 0xf7, 0x16, 0x71, 0x7f, 0x29, 0xe2,
        0x0c, 0x4e, 0x6b, 0x19, 0xc6, 0xea, 0x3b, 0xd3, 0x79, 0x8f, 0x9a, 0xd1, 0xcc, 0xb1, 0xb2,
        0x8a, 0x97,
    ];

    fn header() -> StreamHeader {
        header_of(SALT, PREFIX, SEGMENT_SIZE)
    }

    fn header_of(salt: [u8; 32], nonce_prefix: [u8; 7], segment_size: u32) -> StreamHeader {
        StreamHeader::new(
            salt,
            nonce_prefix,
            SegmentSize::new(segment_size).unwrap(),
            KeyConfirmation::from_bytes([0x33; 32]),
        )
    }

    #[test]
    fn stream_header_roundtrips_through_bytes() {
        let header = StreamHeader::new(
            [0xA5; 32],
            PREFIX,
            SegmentSize::new(SEGMENT_SIZE).unwrap(),
            KeyConfirmation::from_bytes([0x3C; 32]),
        );
        let bytes = header.to_bytes();
        assert_eq!(bytes[..32], [0xA5; 32]);
        assert_eq!(bytes[32..39], PREFIX);
        assert_eq!(bytes[39..43], [0x01, 0x02, 0x03, 0x04]);
        assert_eq!(bytes[43..75], [0x3C; 32]);
        let parsed = StreamHeader::try_from_bytes(&bytes).unwrap();
        assert_eq!(parsed.salt, header.salt);
        assert_eq!(parsed.nonce_prefix, header.nonce_prefix);
        assert_eq!(parsed.segment_size(), header.segment_size());
        assert_eq!(parsed.key_confirmation.0, header.key_confirmation.0);
    }

    #[test]
    fn stream_header_rejects_zero_segment_size() {
        let bytes = [0u8; STREAM_HEADER_LEN];
        assert!(matches!(
            StreamHeader::try_from_bytes(&bytes),
            Err(AeadError::Malformed(_))
        ));
    }

    #[test]
    fn stream_header_rejects_oversized_segment_size() {
        let mut bytes = [0u8; STREAM_HEADER_LEN];
        bytes[39..43].copy_from_slice(&(MAX_SEGMENT_SIZE + 1).to_be_bytes());
        assert!(matches!(
            StreamHeader::try_from_bytes(&bytes),
            Err(AeadError::Malformed(_))
        ));
    }

    #[test]
    fn stream_header_accepts_boundary_segment_sizes() {
        for segment_size in [1, MAX_SEGMENT_SIZE] {
            let bytes = StreamHeader::new(
                SALT,
                PREFIX,
                SegmentSize::new(segment_size).unwrap(),
                KeyConfirmation::from_bytes([0; 32]),
            )
            .to_bytes();
            assert_eq!(
                StreamHeader::try_from_bytes(&bytes)
                    .unwrap()
                    .segment_size()
                    .get(),
                segment_size
            );
        }
    }

    #[test]
    fn key_confirmation_matches_a_fixed_vector() {
        assert_eq!(key_confirmation(b"master_key").0, K_CONFIRM_MASTER_KEY);
    }

    #[test]
    fn stream_header_confirms_the_deriving_key() {
        let header = StreamHeader::new(
            SALT,
            PREFIX,
            SegmentSize::new(SEGMENT_SIZE).unwrap(),
            key_confirmation(b"master_key"),
        );
        assert!(header.confirms_key(b"master_key"));
    }

    #[test]
    fn stream_header_rejects_another_key() {
        let header = StreamHeader::new(
            SALT,
            PREFIX,
            SegmentSize::new(SEGMENT_SIZE).unwrap(),
            key_confirmation(b"master_key"),
        );
        assert!(!header.confirms_key(b"other_key"));
    }

    #[test]
    fn entry_context_layout() {
        let ctx = entry_context(&header(), ChunkType::FHED, b"test_header", b"test_phsf");
        let mut expected = Vec::with_capacity(ENTRY_CONTEXT_LEN);
        expected.extend_from_slice(b"PNA-STREAM-v1");
        expected.extend_from_slice(&Sha256::digest(b"FHEDtest_header"));
        expected.extend_from_slice(&Sha256::digest(b"test_phsf"));
        expected.extend_from_slice(&PREFIX);
        expected.extend_from_slice(&SEGMENT_SIZE.to_be_bytes());

        assert_eq!(ctx.as_slice(), expected);
    }

    #[test]
    fn entry_context_solid_header_hash_includes_shed_type() {
        let ctx = entry_context(&header(), ChunkType::SHED, b"test_header", b"test_phsf");
        let expected = Sha256::digest([b"SHED".as_slice(), b"test_header".as_slice()].concat());
        assert_eq!(&ctx[13..45], expected.as_slice());
    }

    /// Pins every input's contribution and the order they are fed to HKDF. A
    /// differential test cannot: transposing `ikm` with `salt`, or the `FHED` data
    /// with the `PHSF` data, still yields a value that depends on both.
    #[test]
    fn derive_stream_key_matches_a_fixed_vector() {
        assert_eq!(
            derive_stream_key(K_MASTER, &header(), ChunkType::FHED, HEADER_DATA, PHSF_DATA),
            K_STREAM_FHED
        );
    }

    #[test]
    fn segment_nonce_layout() {
        assert_eq!(
            segment_nonce(&[1u8; 7], 0x01020304, false),
            [1, 1, 1, 1, 1, 1, 1, 0x01, 0x02, 0x03, 0x04, 0x00]
        );
        assert_eq!(
            segment_nonce(&[1u8; 7], 0x01020304, true),
            [1, 1, 1, 1, 1, 1, 1, 0x01, 0x02, 0x03, 0x04, 0x01]
        );
    }
}
