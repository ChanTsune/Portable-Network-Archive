use argon2::{Algorithm, Argon2, Params, ParamsBuilder, PasswordHash, PasswordHasher, Version};
use password_hash::SaltString;

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

pub(crate) fn verify_password<'a>(phsf: &'a str, password: &'a str) -> PasswordHash<'a> {
    let mut password_hash = PasswordHash::new(phsf).unwrap();
    match password_hash.algorithm {
        argon2::ARGON2D_IDENT | argon2::ARGON2I_IDENT | argon2::ARGON2ID_IDENT => {
            let a = Argon2::new(
                Algorithm::try_from(password_hash.algorithm).unwrap(),
                Version::try_from(password_hash.version.unwrap()).unwrap(),
                Params::try_from(&password_hash).unwrap(),
            );
            password_hash = a
                .hash_password(password.as_bytes(), password_hash.salt.unwrap().as_str())
                .unwrap();
            password_hash
                .verify_password(&[&a], password.as_bytes())
                .unwrap();
        }
        a => panic!("Unsupported algorithm {:?}", a),
    }
    password_hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random;
    #[test]
    fn verify() {
        let salt = random::salt_string();
        let mut ph = argon2_with_salt("pass", 32, &salt);
        ph.hash.take();
        assert_eq!(ph.hash, None);
        verify_password(&ph.to_string(), "pass");
    }
}
