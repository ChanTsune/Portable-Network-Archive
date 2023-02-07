use argon2::{Argon2, ParamsBuilder};
use password_hash::{PasswordHash, PasswordHasher, SaltString};

pub(crate) fn argon2_with_salt<'a>(
    password: &'a str,
    hash_length: usize,
    salt: &'a SaltString,
) -> PasswordHash<'a> {
    let argon2 = Argon2::from({
        let mut builder = ParamsBuilder::default();
        builder.output_len(hash_length).unwrap();
        builder.params().unwrap()
    });
    argon2.hash_password(password.as_bytes(), salt).unwrap()
}

pub(crate) fn pbkdf2_with_salt<'a>(password: &'a str, salt: &'a SaltString) -> PasswordHash<'a> {
    pbkdf2::Pbkdf2
        .hash_password(password.as_bytes(), salt)
        .unwrap()
}

pub(crate) fn verify_password<'a>(phsf: &'a str, password: &'a str) -> PasswordHash<'a> {
    let mut password_hash = PasswordHash::new(phsf).unwrap();
    match password_hash.algorithm {
        argon2::ARGON2D_IDENT | argon2::ARGON2I_IDENT | argon2::ARGON2ID_IDENT => {
            let a = Argon2::default();
            password_hash = a
                .hash_password_customized(
                    password.as_bytes(),
                    Some(password_hash.algorithm),
                    password_hash.version,
                    argon2::Params::try_from(&password_hash).unwrap(),
                    password_hash.salt.unwrap(),
                )
                .unwrap();
            password_hash
                .verify_password(&[&a], password.as_bytes())
                .unwrap();
        }
        a => {
            if pbkdf2::Algorithm::Pbkdf2Sha256.ident() == a
                || pbkdf2::Algorithm::Pbkdf2Sha512.ident() == a
            {
                password_hash = pbkdf2::Pbkdf2
                    .hash_password_customized(
                        password.as_bytes(),
                        Some(a),
                        password_hash.version,
                        pbkdf2::Params::try_from(&password_hash).unwrap(),
                        password_hash.salt.unwrap(),
                    )
                    .unwrap();
                password_hash
                    .verify_password(&[&pbkdf2::Pbkdf2], password.as_bytes())
                    .unwrap();
            }
        }
        a => panic!("Unsupported algorithm {a:?}"),
    }
    password_hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random;

    #[test]
    fn verify_argon2() {
        let salt = random::salt_string();
        let mut ph = argon2_with_salt("pass", 32, &salt);
        ph.hash.take();
        assert_eq!(ph.hash, None);
        verify_password(&ph.to_string(), "pass");
    }

    #[test]
    fn verify_pbkdf2() {
        let salt = random::salt_string();
        let mut ph = pbkdf2_with_salt("pass", &salt);
        ph.hash.take();
        assert_eq!(ph.hash, None);
        verify_password(&ph.to_string(), "pass");
    }
}
