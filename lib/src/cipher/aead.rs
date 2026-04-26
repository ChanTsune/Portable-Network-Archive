//! Authenticated Encryption with Associated Data (AEAD) for PNA.
//!
//! **Status: prototype skeleton (2026-04-26, post minimal-redesign).**
//! This module is not yet wired into the build. See
//! `lib/src/cipher/aead/README.md` for activation instructions.
//!
//! This module implements the AEAD construction defined in the PNA specification
//! draft `spec/aead-gcm-introduction`:
//!
//! - [§7.4 GCM] — algorithm, parameters, AAD, nonce, key/nonce uniqueness
//! - [§7.5] — FDAT/SDAT layout under AEAD (`12B random nonce || ciphertext || 16B tag`)
//! - [§4.1.5 PHSF] — password-hash output is used directly as the per-entry GCM key
//!
//! [§7.4 GCM]: https://github.com/Portable-Network-Archive/Portable-Network-Archive-Specification/blob/spec/aead-gcm-introduction/cipher_modes/index.md
//! [§7.5]: https://github.com/Portable-Network-Archive/Portable-Network-Archive-Specification/blob/spec/aead-gcm-introduction/cipher_modes/index.md
//! [§4.1.5 PHSF]: https://github.com/Portable-Network-Archive/Portable-Network-Archive-Specification/blob/spec/aead-gcm-introduction/chunk_specifications/index.md
//!
//! # Threat model
//!
//! Addresses finding **CRY-001** (unauthenticated encryption) from the 2026-04-23
//! security audit.
//!
//! # Implementation strategy
//!
//! Per design decision 2026-04-26 (minimal redesign):
//! - AES-256-GCM via `aes-gcm` crate (`Aes256Gcm`).
//! - Camellia-256-GCM via `aes_gcm::AesGcm<Camellia256, U12>` (3-line composition,
//!   leveraging the generic GCM machinery; the crate name is a misnomer).
//! - Per-entry GCM key = PHSF output (Argon2id or PBKDF2, 32 bytes), used directly.
//! - Per-chunk 12-byte nonce generated via CSPRNG; stored inline as the first
//!   12 bytes of each FDAT/SDAT chunk's data field.
//! - AAD = 11 bytes: encryption_byte || mode_byte || entry_index_be ||
//!   chunk_index_be || final_flag.
//! - **No** new chunk types are introduced. **No** additional key derivation
//!   step is applied beyond the entry's PHSF chunk. **No** archive-level
//!   AEAD context state is maintained.

#![allow(dead_code)] // skeleton, not yet wired in

mod aes_gcm;
mod camellia_gcm;
mod read;
mod write;

/// Total AAD length per chunk.
///
/// See spec §7.4.2. Layout (11 bytes):
/// `encryption_byte (1) || mode_byte (1) || entry_index_be (4) || chunk_index_be (4) || final_flag (1)`.
pub(crate) const AEAD_AAD_LEN: usize = 11;

/// Per-chunk nonce length in bytes.
///
/// Required by spec §7.4.1 (96-bit nonces only) and NIST SP 800-38D §5.2.1.1.
pub(crate) const AEAD_NONCE_LEN: usize = 12;

/// GCM authentication tag length in bytes.
///
/// Required by spec §7.4.1 (full 128-bit tags only; truncated tags forbidden).
pub(crate) const AEAD_TAG_LEN: usize = 16;

/// Per-entry AEAD execution context.
///
/// Holds derived state needed for encrypting/decrypting a single entry's chunks.
/// Constructed once per entry from the entry's PHSF chunk; reused for every
/// FDAT/SDAT chunk of that entry.
#[derive(Debug, Clone)]
pub(crate) struct AeadContext {
    /// Per-entry GCM key (32 bytes).
    ///
    /// Equal to the entry's PHSF password-hash output (Argon2id or PBKDF2 output).
    /// No further key derivation is applied. See spec §4.1.5 (PHSF AEAD note).
    pub key: [u8; 32],
    /// FHED `Encryption method` byte (1 = AES, 2 = Camellia).
    pub encryption_byte: u8,
    /// FHED `Cipher mode` byte (= 2 for GCM in PNA Minor 1).
    pub mode_byte: u8,
    /// 0-indexed position of this entry within the archive.
    ///
    /// Authenticated via AAD; defends against intra-archive entry reorder
    /// and cross-entry chunk swap attacks.
    pub entry_index: u32,
}

impl AeadContext {
    /// Construct the 11-byte AAD for a given chunk.
    ///
    /// Per spec §7.4.2:
    /// ```text
    /// AAD[0]      = encryption_byte
    /// AAD[1]      = mode_byte
    /// AAD[2..6]   = entry_index (u32 BE)
    /// AAD[6..10]  = chunk_index (u32 BE)
    /// AAD[10]     = if is_final { 0x01 } else { 0x00 }
    /// ```
    pub(crate) fn build_aad(&self, chunk_index: u32, is_final: bool) -> [u8; AEAD_AAD_LEN] {
        let mut aad = [0u8; AEAD_AAD_LEN];
        aad[0] = self.encryption_byte;
        aad[1] = self.mode_byte;
        aad[2..6].copy_from_slice(&self.entry_index.to_be_bytes());
        aad[6..10].copy_from_slice(&chunk_index.to_be_bytes());
        aad[10] = if is_final { 0x01 } else { 0x00 };
        aad
    }
}

/// Generate a fresh 12-byte nonce from a cryptographically secure RNG.
///
/// Per spec §7.4.3 / §7.4.5: GCM nonces in PNA are random (NIST SP 800-38D
/// §8.2.2 RBG-based construction), one fresh nonce per chunk.
///
/// Encoders MUST surface CSPRNG failure and abort archive creation rather than
/// silently producing weak random values (spec §7.4.5).
pub(crate) fn generate_nonce<R: rand_core::RngCore + rand_core::CryptoRng>(
    rng: &mut R,
) -> [u8; AEAD_NONCE_LEN] {
    let mut nonce = [0u8; AEAD_NONCE_LEN];
    rng.fill_bytes(&mut nonce);
    nonce
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
            encryption_byte: 1, // AES
            mode_byte: 2,       // GCM
            entry_index: 0x0102_0304,
        }
    }

    #[test]
    fn build_aad_length_11_bytes_per_spec_7_4_2() {
        let ctx = dummy_context();
        let aad = ctx.build_aad(0, false);
        assert_eq!(aad.len(), AEAD_AAD_LEN);
        assert_eq!(aad.len(), 11);
    }

    #[test]
    fn build_aad_field_layout_offsets() {
        let ctx = dummy_context();
        let aad = ctx.build_aad(0x0506_0708, false);
        // Offset table per spec §7.4.2 (11 bytes total):
        //   0:      encryption_byte (= 1, AES)
        //   1:      mode_byte (= 2, GCM)
        //   2..6:   entry_index (= 0x0102_0304)
        //   6..10:  chunk_index (= 0x0506_0708)
        //   10:     final flag (0x00 here)
        assert_eq!(aad[0], 1);
        assert_eq!(aad[1], 2);
        assert_eq!(&aad[2..6], &[0x01, 0x02, 0x03, 0x04]);
        assert_eq!(&aad[6..10], &[0x05, 0x06, 0x07, 0x08]);
        assert_eq!(aad[10], 0x00);
    }

    #[test]
    fn build_aad_final_chunk_flag_distinguishes_last_chunk() {
        let ctx = dummy_context();
        let aad_nonfinal = ctx.build_aad(5, false);
        let aad_final = ctx.build_aad(5, true);
        assert_eq!(aad_nonfinal[10], 0x00);
        assert_eq!(aad_final[10], 0x01);
        // All other bytes identical
        assert_eq!(&aad_nonfinal[..10], &aad_final[..10]);
    }

    #[test]
    fn build_aad_entry_index_distinguishes_entries() {
        // Defense against cross-entry chunk swap: same chunk_index in two
        // different entries must produce DIFFERENT AAD.
        let ctx_entry_0 = AeadContext {
            entry_index: 0,
            ..dummy_context()
        };
        let ctx_entry_1 = AeadContext {
            entry_index: 1,
            ..dummy_context()
        };
        let aad_0 = ctx_entry_0.build_aad(5, false);
        let aad_1 = ctx_entry_1.build_aad(5, false);
        assert_ne!(aad_0, aad_1);
        // Diff is in entry_index field (offset 2..6)
        assert_eq!(&aad_0[..2], &aad_1[..2]);
        assert_ne!(&aad_0[2..6], &aad_1[2..6]);
        assert_eq!(&aad_0[6..], &aad_1[6..]);
    }

    #[test]
    fn generate_nonce_returns_12_bytes() {
        // Use a deterministic test RNG to make the assertion reproducible.
        struct TestRng(u8);
        impl rand_core::RngCore for TestRng {
            fn next_u32(&mut self) -> u32 {
                unimplemented!()
            }
            fn next_u64(&mut self) -> u64 {
                unimplemented!()
            }
            fn fill_bytes(&mut self, dest: &mut [u8]) {
                for b in dest {
                    *b = self.0;
                    self.0 = self.0.wrapping_add(1);
                }
            }
            fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
                self.fill_bytes(dest);
                Ok(())
            }
        }
        impl rand_core::CryptoRng for TestRng {}

        let mut rng = TestRng(0xA0);
        let nonce = generate_nonce(&mut rng);
        assert_eq!(nonce.len(), AEAD_NONCE_LEN);
        assert_eq!(
            nonce,
            [
                0xA0, 0xA1, 0xA2, 0xA3, 0xA4, 0xA5, 0xA6, 0xA7, 0xA8, 0xA9, 0xAA, 0xAB
            ]
        );
    }
}
