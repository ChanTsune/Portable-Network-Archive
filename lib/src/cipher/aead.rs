//! Authenticated Encryption with Associated Data (AEAD) for PNA.
//!
//! **Status: prototype skeleton (2026-04-25, updated post-spec-finalization).**
//! This module is not yet wired into the build. See
//! `lib/src/cipher/aead/README.md` for activation instructions.
//!
//! This module implements the AEAD construction defined in the PNA specification
//! draft `spec/aead-gcm-introduction`:
//!
//! - [§7.4 GCM] — algorithm, parameters, AAD, nonce, key/nonce uniqueness
//! - [§7.5] — FDAT/SDAT layout under AEAD (`ciphertext || 16-byte tag`)
//! - [§8.3 HKDF] — per-entry subkey derivation
//! - [§4.1.x AENC] — per-archive encryption context chunk
//! - [§4.1.x FENC] — per-entry encryption context chunk
//!
//! [§7.4 GCM]: https://github.com/Portable-Network-Archive/Portable-Network-Archive-Specification/blob/spec/aead-gcm-introduction/cipher_modes/index.md
//! [§7.5]: https://github.com/Portable-Network-Archive/Portable-Network-Archive-Specification/blob/spec/aead-gcm-introduction/cipher_modes/index.md
//! [§8.3 HKDF]: https://github.com/Portable-Network-Archive/Portable-Network-Archive-Specification/blob/spec/aead-gcm-introduction/key_derivation_algorithms/index.md
//! [§4.1.x AENC]: https://github.com/Portable-Network-Archive/Portable-Network-Archive-Specification/blob/spec/aead-gcm-introduction/chunk_specifications/index.md
//! [§4.1.x FENC]: https://github.com/Portable-Network-Archive/Portable-Network-Archive-Specification/blob/spec/aead-gcm-introduction/chunk_specifications/index.md
//!
//! # Threat model
//!
//! Addresses finding **CRY-001** (unauthenticated encryption) from the 2026-04-23
//! security audit.
//!
//! # Implementation strategy
//!
//! Per design decision 2026-04-25:
//! - AES-256-GCM via `aes-gcm` crate (`Aes256Gcm`).
//! - Camellia-256-GCM via `aes_gcm::AesGcm<Camellia256, U12>` (3-line composition,
//!   leveraging the generic GCM machinery; the crate name is a misnomer).

#![allow(dead_code)] // skeleton, not yet wired in

mod aes_gcm;
mod camellia_gcm;
mod read;
mod write;

/// AAD construction magic constant (11 bytes, no NUL).
///
/// See spec §7.4.2. Fixed for the AEAD framework_version=1; future framework
/// versions MUST use a different magic.
pub(crate) const AEAD_AAD_MAGIC: &[u8; 11] = b"PNA-AEAD-v1";

/// Total AAD length per chunk for framework_version = 1.
///
/// Layout: 11 (magic) + 1 (encryption_byte) + 1 (mode_byte) + 16 (archive_id)
/// + 16 (entry_random) + 4 (entry_index) + 4 (chunk_index) + 1 (final_flag)
/// + 32 (metadata_hash) = **86 bytes**.
pub(crate) const AEAD_AAD_LEN: usize = 86;

/// AEAD framework version supported by this implementation.
///
/// Matches the `framework_version` field of the `AENC` chunk (spec §4.1.x).
pub(crate) const AEAD_FRAMEWORK_VERSION: u8 = 1;

/// Per-archive encryption context (carried in `AENC` chunk).
///
/// See spec §4.1.x AENC. For framework_version=1 + cipher_mode_id=2 (GCM),
/// the payload is exactly 16 bytes of CSPRNG-generated `archive_identifier`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ArchiveContext {
    /// FHED `Cipher mode` value this AENC chunk applies to (e.g., 2 for GCM).
    pub cipher_mode_id: u8,
    /// AEAD framework version (currently always [`AEAD_FRAMEWORK_VERSION`]).
    pub framework_version: u8,
    /// 16-byte archive-unique random identifier from CSPRNG.
    pub archive_identifier: [u8; 16],
}

/// Per-entry encryption context (carried in `FENC` chunk).
///
/// See spec §4.1.x FENC. For an entry with `Cipher mode = 2` (GCM) under
/// framework_version=1, the payload is exactly 16 bytes of CSPRNG-generated
/// `entry_random`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EntryContext {
    /// 16-byte per-entry random from CSPRNG. Used as additional HKDF salt
    /// input for defense-in-depth.
    pub entry_random: [u8; 16],
}

/// Per-entry AEAD execution context.
///
/// Holds derived state needed for encrypting/decrypting a single entry's chunks.
/// Constructed once per entry (after HKDF key derivation); reused for every
/// FDAT/SDAT chunk of that entry.
#[derive(Debug)]
pub(crate) struct AeadContext {
    /// Per-entry derived key (32 bytes from HKDF-SHA-256). See spec §8.3.
    pub key: [u8; 32],
    /// Copy of archive context for AAD construction.
    pub archive: ArchiveContext,
    /// Copy of entry context for AAD construction.
    pub entry: EntryContext,
    /// FHED `Encryption method` byte (1 = AES, 2 = Camellia).
    pub encryption_byte: u8,
    /// FHED `Cipher mode` byte (= 2 for GCM in framework_version=1).
    pub mode_byte: u8,
    /// 0-indexed position of this entry within the archive.
    pub entry_index: u32,
    /// SHA-256 hash of FHED + ancillary metadata chunks. See spec §7.4.2.
    pub metadata_hash: [u8; 32],
}

impl AeadContext {
    /// Construct the 96-bit nonce for a given chunk index.
    ///
    /// Per spec §7.4.3:
    /// ```text
    /// nonce[0..8]   = archive.archive_identifier[0..8]
    /// nonce[8..12]  = chunk_index (u32 BE)
    /// ```
    #[inline]
    pub(crate) fn build_nonce(&self, chunk_index: u32) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        nonce[..8].copy_from_slice(&self.archive.archive_identifier[..8]);
        nonce[8..12].copy_from_slice(&chunk_index.to_be_bytes());
        nonce
    }

    /// Construct the 86-byte AAD for a given chunk.
    ///
    /// Per spec §7.4.2 (framework_version=1).
    pub(crate) fn build_aad(&self, chunk_index: u32, is_final: bool) -> [u8; AEAD_AAD_LEN] {
        let mut aad = [0u8; AEAD_AAD_LEN];
        let mut off = 0;

        // 11 bytes: magic
        aad[off..off + 11].copy_from_slice(AEAD_AAD_MAGIC);
        off += 11;

        // 1 byte: encryption_byte
        aad[off] = self.encryption_byte;
        off += 1;

        // 1 byte: mode_byte
        aad[off] = self.mode_byte;
        off += 1;

        // 16 bytes: archive_identifier
        aad[off..off + 16].copy_from_slice(&self.archive.archive_identifier);
        off += 16;

        // 16 bytes: entry_random
        aad[off..off + 16].copy_from_slice(&self.entry.entry_random);
        off += 16;

        // 4 bytes: entry_index (u32 BE)
        aad[off..off + 4].copy_from_slice(&self.entry_index.to_be_bytes());
        off += 4;

        // 4 bytes: chunk_index (u32 BE)
        aad[off..off + 4].copy_from_slice(&chunk_index.to_be_bytes());
        off += 4;

        // 1 byte: is_final_chunk
        aad[off] = if is_final { 0x01 } else { 0x00 };
        off += 1;

        // 32 bytes: metadata_hash
        aad[off..off + 32].copy_from_slice(&self.metadata_hash);
        off += 32;

        debug_assert_eq!(off, AEAD_AAD_LEN);
        aad
    }
}

/// AEAD authentication failure error.
///
/// Distinct from I/O errors and format errors per spec §12.3.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AuthFailure;

impl core::fmt::Display for AuthFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("AEAD authentication tag verification failed")
    }
}

impl std::error::Error for AuthFailure {}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_context() -> AeadContext {
        AeadContext {
            key: [0u8; 32],
            archive: ArchiveContext {
                cipher_mode_id: 2,
                framework_version: 1,
                archive_identifier: [
                    0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8,
                    0xB1, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8,
                ],
            },
            entry: EntryContext {
                entry_random: [
                    0xC1, 0xC2, 0xC3, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8,
                    0xD1, 0xD2, 0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8,
                ],
            },
            encryption_byte: 1, // AES
            mode_byte: 2,       // GCM
            entry_index: 0x0102_0304,
            metadata_hash: [0xEE; 32],
        }
    }

    #[test]
    fn build_nonce_layout_per_spec_7_4_3() {
        let ctx = dummy_context();
        let nonce = ctx.build_nonce(0x0506_0708);
        // First 8 bytes are archive_identifier[0..8]
        assert_eq!(&nonce[..8], &[0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8]);
        // Last 4 bytes are chunk_index BE
        assert_eq!(&nonce[8..12], &[0x05, 0x06, 0x07, 0x08]);
    }

    #[test]
    fn build_aad_length_86_bytes_per_spec_7_4_2() {
        let ctx = dummy_context();
        let aad = ctx.build_aad(0, false);
        assert_eq!(aad.len(), AEAD_AAD_LEN);
        assert_eq!(aad.len(), 86);
    }

    #[test]
    fn build_aad_starts_with_magic() {
        let ctx = dummy_context();
        let aad = ctx.build_aad(0, false);
        assert_eq!(&aad[..11], AEAD_AAD_MAGIC);
        assert_eq!(&aad[..11], b"PNA-AEAD-v1");
    }

    #[test]
    fn build_aad_final_chunk_flag() {
        let ctx = dummy_context();
        let aad_nonfinal = ctx.build_aad(5, false);
        let aad_final = ctx.build_aad(5, true);
        // The final flag byte is at offset 11+1+1+16+16+4+4 = 53
        assert_eq!(aad_nonfinal[53], 0x00);
        assert_eq!(aad_final[53], 0x01);
        // All other bytes identical
        assert_eq!(&aad_nonfinal[..53], &aad_final[..53]);
        assert_eq!(&aad_nonfinal[54..], &aad_final[54..]);
    }

    #[test]
    fn build_aad_field_layout_offsets() {
        let ctx = dummy_context();
        let aad = ctx.build_aad(0x0506_0708, false);
        // Offset table per spec §7.4.2:
        //   0..11:  magic
        //   11:     encryption_byte (= 1)
        //   12:     mode_byte (= 2)
        //   13..29: archive_identifier (16 bytes)
        //   29..45: entry_random (16 bytes)
        //   45..49: entry_index (= 0x0102_0304)
        //   49..53: chunk_index (= 0x0506_0708)
        //   53:     is_final_chunk
        //   54..86: metadata_hash
        assert_eq!(aad[11], 1); // AES
        assert_eq!(aad[12], 2); // GCM
        assert_eq!(&aad[13..29], &ctx.archive.archive_identifier);
        assert_eq!(&aad[29..45], &ctx.entry.entry_random);
        assert_eq!(&aad[45..49], &[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(&aad[49..53], &[0x05, 0x06, 0x07, 0x08]);
        assert_eq!(aad[53], 0x00); // not final
        assert_eq!(&aad[54..86], &[0xEE; 32]);
    }
}
