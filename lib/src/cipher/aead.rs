//! AEAD (Authenticated Encryption with Associated Data) support for PNA archives.
//!
//! This module provides cryptographic utilities for AEAD cipher modes,
//! including key derivation functions for entropy-based key material.

use hkdf::Hkdf;
use sha2::Sha256;

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
#[allow(dead_code)]
pub(crate) fn hkdf_sha256(ikm: &[u8], salt: &[u8], info: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(salt), ikm);
    let mut okm = [0u8; 32];
    hk.expand(info, &mut okm)
        .expect("32 bytes is a valid HKDF-SHA-256 output length");
    okm
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
    fn hkdf_sha256_rfc5869_test_case_1() {
        let ikm = [0x0bu8; 22];
        let salt: Vec<u8> = (0x00u8..=0x0c).collect();
        let info: Vec<u8> = (0xf0u8..=0xf9).collect();
        assert_eq!(
            hkdf_sha256(&ikm, &salt, &info).as_slice(),
            &OKM_42_BYTES[..32]
        );
    }
}
