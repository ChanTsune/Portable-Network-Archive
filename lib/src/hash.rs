//! Password hashing helpers.
use argon2::{Argon2, ParamsBuilder, Version};
use password_hash::{PasswordHash, PasswordHasher, SaltString};
use std::io;

pub(crate) fn argon2_with_salt<'a>(
    password: &'a [u8],
    algorithm: argon2::Algorithm,
    time_cost: Option<u32>,
    memory_cost: Option<u32>,
    parallelism_cost: Option<u32>,
    hash_length: usize,
    salt: &'a SaltString,
) -> io::Result<PasswordHash<'a>> {
    let mut builder = ParamsBuilder::default();
    if let Some(time_cost) = time_cost {
        builder.t_cost(time_cost);
    };
    if let Some(memory_cost) = memory_cost {
        builder.m_cost(memory_cost);
    };
    if let Some(parallelism_cost) = parallelism_cost {
        builder.p_cost(parallelism_cost);
    };
    let argon2 = builder
        .output_len(hash_length)
        .context(algorithm, Version::default())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    argon2
        .hash_password(password, salt)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub(crate) fn pbkdf2_with_salt<'a>(
    password: &'a [u8],
    algorithm: pbkdf2::Algorithm,
    params: pbkdf2::Params,
    salt: &'a SaltString,
) -> io::Result<PasswordHash<'a>> {
    pbkdf2::Pbkdf2
        .hash_password_customized(password, Some(algorithm.ident()), None, params, salt)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

/// Key size for HashedPassword: 32 bytes (AES-256 / Camellia-256).
const HASHED_PASSWORD_KEY_SIZE: usize = 32;

pub(crate) fn new_hashed_password(
    password: &[u8],
    hash_algorithm: crate::HashAlgorithm,
) -> io::Result<crate::HashedPassword> {
    use crate::entry::HashAlgorithmParams;

    let salt = crate::random::salt_string();
    let mut password_hash = match hash_algorithm.0 {
        HashAlgorithmParams::Argon2Id {
            time_cost,
            memory_cost,
            parallelism_cost,
        } => argon2_with_salt(
            password,
            argon2::Algorithm::Argon2id,
            time_cost,
            memory_cost,
            parallelism_cost,
            HASHED_PASSWORD_KEY_SIZE,
            &salt,
        ),
        HashAlgorithmParams::Pbkdf2Sha256 { rounds } => {
            let mut params = pbkdf2::Params {
                output_length: HASHED_PASSWORD_KEY_SIZE,
                ..Default::default()
            };
            if let Some(rounds) = rounds {
                params.rounds = rounds;
            }
            pbkdf2_with_salt(password, pbkdf2::Algorithm::Pbkdf2Sha256, params, &salt)
        }
    }?;
    let key = password_hash
        .hash
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "Failed to get hash"))?;
    let phsf = password_hash.to_string();
    Ok(crate::HashedPassword { key, phsf })
}

pub(crate) fn derive_password_hash<'a>(
    phsf: &'a str,
    password: &'a [u8],
) -> io::Result<PasswordHash<'a>> {
    let password_hash =
        PasswordHash::new(phsf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let salt = password_hash.salt.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing salt in password hash")
    })?;
    match password_hash.algorithm {
        argon2::ARGON2D_IDENT | argon2::ARGON2I_IDENT | argon2::ARGON2ID_IDENT => {
            let argon2 = Argon2::default();
            argon2
                .hash_password_customized(
                    password,
                    Some(password_hash.algorithm),
                    password_hash.version,
                    argon2::Params::try_from(&password_hash)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                    salt,
                )
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
        pbkdf2::Algorithm::PBKDF2_SHA256_IDENT | pbkdf2::Algorithm::PBKDF2_SHA512_IDENT => {
            pbkdf2::Pbkdf2
                .hash_password_customized(
                    password,
                    Some(password_hash.algorithm),
                    password_hash.version,
                    pbkdf2::Params::try_from(&password_hash)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                    salt,
                )
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
        }
        a => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unsupported algorithm {a:?}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HashAlgorithm, HashedPassword, random};
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn derive_argon2() {
        let salt = random::salt_string();
        let mut ph = argon2_with_salt(
            b"pass",
            argon2::Algorithm::Argon2id,
            None,
            None,
            None,
            32,
            &salt,
        )
        .unwrap();
        ph.hash.take();
        assert_eq!(ph.hash, None);
        let ps = ph.to_string();
        let ph = derive_password_hash(&ps, b"pass").unwrap();
        assert!(ph.hash.is_some());
    }

    #[test]
    fn derive_pbkdf2() {
        let salt = random::salt_string();
        let mut ph = pbkdf2_with_salt(
            b"pass",
            pbkdf2::Algorithm::Pbkdf2Sha256,
            pbkdf2::Params::default(),
            &salt,
        )
        .unwrap();
        ph.hash.take();
        assert_eq!(ph.hash, None);
        let ps = ph.to_string();
        let ph = derive_password_hash(&ps, b"pass").unwrap();
        assert!(ph.hash.is_some());
    }

    #[test]
    fn hashed_password_argon2() {
        let hp = HashedPassword::new(b"password", HashAlgorithm::argon2id()).unwrap();
        assert_eq!(hp.key.len(), 32);
        assert!(hp.phsf.starts_with("$argon2id$"));
    }

    #[test]
    fn hashed_password_pbkdf2() {
        let hp = HashedPassword::new(b"password", HashAlgorithm::pbkdf2_sha256()).unwrap();
        assert_eq!(hp.key.len(), 32);
        assert!(hp.phsf.starts_with("$pbkdf2-sha256$"));
    }

    #[test]
    fn hashed_password_debug_redacts_key() {
        let hp = HashedPassword::new(b"password", HashAlgorithm::argon2id()).unwrap();
        let debug = format!("{hp:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("password"));
        // PHSF must also be redacted — should not contain algorithm identifier
        assert!(!debug.contains("$argon2id$"));
    }
}
