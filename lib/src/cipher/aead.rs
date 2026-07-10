//! AEAD (Authenticated Encryption with Associated Data) support for PNA archives.
//!
//! Provides the key- and nonce-derivation primitives for cipher mode 2 (GCM
//! STREAM). HKDF-SHA-256 derives a per-stream key from the master key, the
//! stream salt, and an entry context that binds the raw `FHED`/`SHED` Type and
//! Data fields plus the `PHSF` Data field; each segment's 96-bit nonce is
//! derived from the stream's nonce prefix, a segment counter, and a
//! final-segment marker.

use crate::{ChunkType, error::AeadError};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};

pub(crate) const STREAM_HEADER_LEN: usize = 43;
pub(crate) const GCM_TAG_LEN: usize = 16;
pub(crate) const MAX_SEGMENT_SIZE: u32 = 67_108_864; // 64 MiB
pub(crate) const DEFAULT_SEGMENT_SIZE: u32 = 1_048_576; // 1 MiB
const DOMAIN_TAG: &[u8; 13] = b"PNA-STREAM-v1";

/// On-wire GCM stream header: `salt(32) || nonce_prefix(7) || segment_size(u32 BE)`.
#[derive(Debug)]
pub(crate) struct StreamHeader {
    pub(crate) salt: [u8; 32],
    pub(crate) nonce_prefix: [u8; 7],
    segment_size: u32,
}

impl StreamHeader {
    /// Constructs a header, rejecting an out-of-range segment size.
    ///
    /// This is the only way to construct a `StreamHeader`, so every instance
    /// in memory is guaranteed to carry a validated segment size.
    pub(crate) fn new(
        salt: [u8; 32],
        nonce_prefix: [u8; 7],
        segment_size: u32,
    ) -> Result<Self, AeadError> {
        if segment_size == 0 || segment_size > MAX_SEGMENT_SIZE {
            return Err(AeadError::Malformed("segment size out of range"));
        }
        Ok(Self {
            salt,
            nonce_prefix,
            segment_size,
        })
    }

    pub(crate) fn segment_size(&self) -> u32 {
        self.segment_size
    }

    pub(crate) fn to_bytes(&self) -> [u8; STREAM_HEADER_LEN] {
        let mut bytes = [0u8; STREAM_HEADER_LEN];
        bytes[..32].copy_from_slice(&self.salt);
        bytes[32..39].copy_from_slice(&self.nonce_prefix);
        bytes[39..43].copy_from_slice(&self.segment_size.to_be_bytes());
        bytes
    }

    pub(crate) fn try_from_bytes(bytes: &[u8; STREAM_HEADER_LEN]) -> Result<Self, AeadError> {
        let segment_size = u32::from_be_bytes(bytes[39..43].try_into().unwrap());
        Self::new(
            bytes[..32].try_into().unwrap(),
            bytes[32..39].try_into().unwrap(),
            segment_size,
        )
    }
}

/// Derive output key material using HKDF-SHA-256.
///
/// Implements RFC 5869 HKDF (HMAC-based Extract-and-Expand Key Derivation Function)
/// with SHA-256 as the hash algorithm.
///
/// # Arguments
///
/// * `ikm` - Input keying material
/// * `salt` - Salt for the extraction step (can be empty)
/// * `info` - Application-specific context information (can be empty)
///
/// # Returns
///
/// A 32-byte output keying material array.
pub(crate) fn hkdf_sha256(ikm: &[u8], salt: &[u8], info: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(salt), ikm);
    let mut okm = [0u8; 32];
    hk.expand(info, &mut okm)
        .expect("32 bytes is a valid HKDF-SHA-256 output length");
    okm
}

pub(crate) fn entry_context(
    header_chunk_type: ChunkType,
    header_chunk_data: &[u8],
    phsf_chunk_data: &[u8],
) -> [u8; 77] {
    let mut ctx = [0u8; 77];
    let mut header_hasher = Sha256::new();
    header_hasher.update(header_chunk_type.as_bytes());
    header_hasher.update(header_chunk_data);

    ctx[..13].copy_from_slice(DOMAIN_TAG);
    ctx[13..45].copy_from_slice(&header_hasher.finalize());
    ctx[45..77].copy_from_slice(&Sha256::digest(phsf_chunk_data));
    ctx
}

pub(crate) fn derive_stream_key(
    k_master: &[u8],
    stream_salt: &[u8; 32],
    header_chunk_type: ChunkType,
    header_chunk_data: &[u8],
    phsf_chunk_data: &[u8],
) -> [u8; 32] {
    let info = entry_context(header_chunk_type, header_chunk_data, phsf_chunk_data);
    hkdf_sha256(k_master, stream_salt, &info)
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

    const OKM_42_BYTES: [u8; 42] = [
        0x3c, 0xb2, 0x5f, 0x25, 0xfa, 0xac, 0xd5, 0x7a, 0x90, 0x43, 0x4f, 0x64, 0xd0, 0x36, 0x2f,
        0x2a, 0x2d, 0x2d, 0x0a, 0x90, 0xcf, 0x1a, 0x5a, 0x4c, 0x5d, 0xb0, 0x2d, 0x56, 0xec, 0xc4,
        0xc5, 0xbf, 0x34, 0x00, 0x72, 0x08, 0xd5, 0xb8, 0x87, 0x18, 0x58, 0x65,
    ];

    #[test]
    fn stream_header_roundtrips_through_bytes() {
        let header = StreamHeader::new([0xA5; 32], [0x5A; 7], 0x01020304).unwrap();
        let bytes = header.to_bytes();
        assert_eq!(bytes[..32], [0xA5; 32]);
        assert_eq!(bytes[32..39], [0x5A; 7]);
        assert_eq!(bytes[39..43], [0x01, 0x02, 0x03, 0x04]);
        let parsed = StreamHeader::try_from_bytes(&bytes).unwrap();
        assert_eq!(parsed.salt, header.salt);
        assert_eq!(parsed.nonce_prefix, header.nonce_prefix);
        assert_eq!(parsed.segment_size(), header.segment_size());
    }

    #[test]
    fn stream_header_new_rejects_zero_segment_size() {
        assert!(matches!(
            StreamHeader::new([0; 32], [0; 7], 0),
            Err(AeadError::Malformed(_))
        ));
    }

    #[test]
    fn stream_header_new_rejects_oversized_segment_size() {
        assert!(matches!(
            StreamHeader::new([0; 32], [0; 7], MAX_SEGMENT_SIZE + 1),
            Err(AeadError::Malformed(_))
        ));
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
            let bytes = StreamHeader::new([0; 32], [0; 7], segment_size)
                .unwrap()
                .to_bytes();
            assert_eq!(
                StreamHeader::try_from_bytes(&bytes).unwrap().segment_size(),
                segment_size
            );
        }
    }

    #[test]
    fn hkdf_sha256_rfc5869_test_case_1() {
        let ikm = [0x0bu8; 22];
        let salt: Vec<u8> = (0x00u8..=0x0c).collect();
        let info: Vec<u8> = (0xf0u8..=0xf9).collect();
        assert_eq!(
            hkdf_sha256(&ikm, &salt, &info).as_slice(),
            &OKM_42_BYTES[..32]
        );
    }

    #[test]
    fn entry_context_is_77_bytes() {
        let header = b"test_header";
        let phsf = b"test_phsf";
        let ctx = entry_context(ChunkType::FHED, header, phsf);
        let mut expected = Vec::with_capacity(77);
        expected.extend_from_slice(b"PNA-STREAM-v1");
        expected.extend_from_slice(&Sha256::digest(b"FHEDtest_header"));
        expected.extend_from_slice(&Sha256::digest(phsf));

        assert_eq!(ctx.len(), 77);
        assert_eq!(ctx.as_slice(), expected);
    }

    #[test]
    fn entry_context_domain_tag() {
        let header = b"test_header";
        let phsf = b"test_phsf";
        let ctx = entry_context(ChunkType::FHED, header, phsf);
        assert_eq!(&ctx[..13], b"PNA-STREAM-v1");
    }

    #[test]
    fn entry_context_per_entry_header_hash_includes_fhed_type() {
        let header = b"test_header";
        let phsf = b"test_phsf";
        let ctx = entry_context(ChunkType::FHED, header, phsf);
        let expected_header_hash = Sha256::digest([b"FHED".as_slice(), header.as_slice()].concat());
        assert_eq!(&ctx[13..45], expected_header_hash.as_slice());
    }

    #[test]
    fn entry_context_solid_header_hash_includes_shed_type() {
        let header = b"test_header";
        let phsf = b"test_phsf";
        let ctx = entry_context(ChunkType::SHED, header, phsf);
        let expected_header_hash = Sha256::digest([b"SHED".as_slice(), header.as_slice()].concat());
        assert_eq!(&ctx[13..45], expected_header_hash.as_slice());
    }

    #[test]
    fn entry_context_header_hash_excludes_length_and_crc() {
        let header = b"test_header";
        let phsf = b"test_phsf";
        let ctx = entry_context(ChunkType::FHED, header, phsf);
        let expected_header_hash = Sha256::digest(b"FHEDtest_header");
        assert_eq!(&ctx[13..45], expected_header_hash.as_slice());
    }

    #[test]
    fn entry_context_phsf_hash_uses_data_only() {
        let header = b"test_header";
        let phsf = b"test_phsf";
        let ctx = entry_context(ChunkType::FHED, header, phsf);
        let expected_phsf_hash = Sha256::digest(phsf);
        assert_eq!(&ctx[45..77], expected_phsf_hash.as_slice());
    }

    #[test]
    fn derive_stream_key_output_length() {
        let k_master = b"master_key";
        let stream_salt = [0x42u8; 32];
        let header = b"header";
        let phsf = b"phsf";
        let key = derive_stream_key(k_master, &stream_salt, ChunkType::FHED, header, phsf);
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn derive_stream_key_sensitive_to_k_master() {
        let stream_salt = [0x42u8; 32];
        let header = b"header";
        let phsf = b"phsf";
        let key1 = derive_stream_key(b"master_key_1", &stream_salt, ChunkType::FHED, header, phsf);
        let mut master_key_2 = b"master_key_1".to_vec();
        master_key_2[0] = master_key_2[0].wrapping_add(1);
        let key2 = derive_stream_key(&master_key_2, &stream_salt, ChunkType::FHED, header, phsf);
        assert_ne!(key1, key2);
    }

    #[test]
    fn derive_stream_key_sensitive_to_stream_salt() {
        let k_master = b"master_key";
        let header = b"header";
        let phsf = b"phsf";
        let salt1 = [0x42u8; 32];
        let mut salt2 = [0x42u8; 32];
        salt2[0] = salt2[0].wrapping_add(1);
        let key1 = derive_stream_key(k_master, &salt1, ChunkType::FHED, header, phsf);
        let key2 = derive_stream_key(k_master, &salt2, ChunkType::FHED, header, phsf);
        assert_ne!(key1, key2);
    }

    #[test]
    fn derive_stream_key_sensitive_to_header_chunk_type() {
        let k_master = b"master_key";
        let stream_salt = [0x42u8; 32];
        let header = b"header";
        let phsf = b"phsf";
        let key1 = derive_stream_key(k_master, &stream_salt, ChunkType::FHED, header, phsf);
        let key2 = derive_stream_key(k_master, &stream_salt, ChunkType::SHED, header, phsf);
        assert_ne!(key1, key2);
    }

    #[test]
    fn derive_stream_key_sensitive_to_header_chunk_data() {
        let k_master = b"master_key";
        let stream_salt = [0x42u8; 32];
        let phsf = b"phsf";
        let key1 = derive_stream_key(k_master, &stream_salt, ChunkType::FHED, b"header1", phsf);
        let key2 = derive_stream_key(k_master, &stream_salt, ChunkType::FHED, b"header2", phsf);
        assert_ne!(key1, key2);
    }

    #[test]
    fn derive_stream_key_sensitive_to_phsf_chunk_data() {
        let k_master = b"master_key";
        let stream_salt = [0x42u8; 32];
        let header = b"header";
        let key1 = derive_stream_key(k_master, &stream_salt, ChunkType::FHED, header, b"phsf1");
        let key2 = derive_stream_key(k_master, &stream_salt, ChunkType::FHED, header, b"phsf2");
        assert_ne!(key1, key2);
    }

    #[test]
    fn segment_nonce_layout() {
        let prefix = [1u8; 7];
        let counter = 0x01020304u32;
        let nonce = segment_nonce(&prefix, counter, false);
        assert_eq!(nonce[..7], prefix);
        assert_eq!(&nonce[7..11], &counter.to_be_bytes());
        assert_eq!(nonce[11], 0x00);
    }

    #[test]
    fn segment_nonce_exact_case() {
        let prefix = [1u8; 7];
        let counter = 0x01020304u32;
        let nonce = segment_nonce(&prefix, counter, false);
        let expected = [1u8, 1, 1, 1, 1, 1, 1, 0x01, 0x02, 0x03, 0x04, 0x00];
        assert_eq!(nonce, expected);
    }

    #[test]
    fn segment_nonce_final_flag_set() {
        let prefix = [1u8; 7];
        let counter = 0x01020304u32;
        let nonce = segment_nonce(&prefix, counter, true);
        assert_eq!(nonce[11], 0x01);
    }

    #[test]
    fn segment_nonce_final_flag_unset() {
        let prefix = [1u8; 7];
        let counter = 0x01020304u32;
        let nonce = segment_nonce(&prefix, counter, false);
        assert_eq!(nonce[11], 0x00);
    }

    #[test]
    fn segment_nonce_counter_zero() {
        let prefix = [0xFFu8; 7];
        let counter = 0u32;
        let nonce = segment_nonce(&prefix, counter, false);
        assert_eq!(&nonce[7..11], &[0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn segment_nonce_counter_max() {
        let prefix = [0xFFu8; 7];
        let counter = u32::MAX;
        let nonce = segment_nonce(&prefix, counter, false);
        assert_eq!(&nonce[7..11], &[0xFF, 0xFF, 0xFF, 0xFF]);
    }
}
