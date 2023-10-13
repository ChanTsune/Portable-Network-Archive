use argon2::{Argon2, ParamsBuilder, Version};
use password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use std::io;

pub(crate) fn argon2_with_salt<'a>(
    password: &'a str,
    algorithm: argon2::Algorithm,
    hash_length: usize,
    salt: &'a SaltString,
) -> io::Result<PasswordHash<'a>> {
    let mut builder = ParamsBuilder::default();
    let argon2 = builder
        .output_len(hash_length)
        .context(algorithm, Version::default())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    argon2
        .hash_password(password.as_bytes(), salt)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub(crate) fn pbkdf2_with_salt<'a>(
    password: &'a str,
    algorithm: pbkdf2::Algorithm,
    params: pbkdf2::Params,
    salt: &'a SaltString,
) -> io::Result<PasswordHash<'a>> {
    pbkdf2::Pbkdf2
        .hash_password_customized(
            password.as_bytes(),
            Some(algorithm.ident()),
            None,
            params,
            salt,
        )
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub(crate) fn verify_password<'a>(
    phsf: &'a str,
    password: &'a str,
) -> io::Result<PasswordHash<'a>> {
    let mut password_hash =
        PasswordHash::new(phsf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    match password_hash.algorithm {
        argon2::ARGON2D_IDENT | argon2::ARGON2I_IDENT | argon2::ARGON2ID_IDENT => {
            let argon2 = Argon2::default();
            password_hash = argon2
                .hash_password_customized(
                    password.as_bytes(),
                    Some(password_hash.algorithm),
                    password_hash.version,
                    argon2::Params::try_from(&password_hash)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                    password_hash.salt.unwrap(),
                )
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            argon2
                .verify_password(password.as_bytes(), &password_hash)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        }
        pbkdf2::Algorithm::PBKDF2_SHA256_IDENT | pbkdf2::Algorithm::PBKDF2_SHA512_IDENT => {
            password_hash = pbkdf2::Pbkdf2
                .hash_password_customized(
                    password.as_bytes(),
                    Some(password_hash.algorithm),
                    password_hash.version,
                    pbkdf2::Params::try_from(&password_hash)
                        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
                    password_hash.salt.unwrap(),
                )
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            pbkdf2::Pbkdf2
                .verify_password(password.as_bytes(), &password_hash)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        }
        a => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("Unsupported algorithm {a:?}"),
            ))
        }
    }
    Ok(password_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random;

    #[test]
    fn verify_argon2() {
        let salt = random::salt_string();
        let mut ph = argon2_with_salt("pass", argon2::Algorithm::Argon2id, 32, &salt).unwrap();
        ph.hash.take();
        assert_eq!(ph.hash, None);
        verify_password(&ph.to_string(), "pass").unwrap();
    }

    #[test]
    fn verify_pbkdf2() {
        let salt = random::salt_string();
        let mut ph = pbkdf2_with_salt(
            "pass",
            pbkdf2::Algorithm::Pbkdf2Sha256,
            pbkdf2::Params::default(),
            &salt,
        )
        .unwrap();
        ph.hash.take();
        assert_eq!(ph.hash, None);
        verify_password(&ph.to_string(), "pass").unwrap();
    }
}
