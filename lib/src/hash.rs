//! Password hashing helpers.
use argon2::{Argon2, ParamsBuilder, Version};
use password_hash::{Output, PasswordHash, PasswordHasher, SaltString};
use std::io;

// `pbkdf2 0.13` is wired to `password_hash 0.6`, while `argon2 0.5.3` and the
// rest of libpna are anchored to `password_hash 0.5`. Bridge the two through
// PHC strings, which are stable across crate versions.
use pbkdf2::password_hash as ph6;

#[inline]
fn invalid_data<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

pub(crate) fn argon2_with_salt(
    password: &[u8],
    algorithm: argon2::Algorithm,
    time_cost: Option<u32>,
    memory_cost: Option<u32>,
    parallelism_cost: Option<u32>,
    hash_length: usize,
    salt: &SaltString,
) -> io::Result<(Output, String)> {
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
    let pwhash = argon2.hash_password(password, salt).map_err(invalid_data)?;
    finalize(pwhash.hash, pwhash.to_string())
}

pub(crate) fn pbkdf2_with_salt(
    password: &[u8],
    algorithm: pbkdf2::Algorithm,
    params: pbkdf2::Params,
    salt: &SaltString,
) -> io::Result<(Output, String)> {
    use ph6::CustomizedPasswordHasher;
    let pwhash = pbkdf2::Pbkdf2::default()
        .hash_password_customized(
            password,
            salt.as_str().as_bytes(),
            Some(algorithm.as_ref()),
            None,
            params,
        )
        .map_err(invalid_data)?;
    let phc_string = pwhash.to_string();
    let raw = pwhash
        .hash
        .ok_or_else(|| io::Error::other("pbkdf2 returned no hash"))?;
    Ok((Output::new(raw.as_ref()).map_err(invalid_data)?, phc_string))
}

#[inline]
fn finalize(hash: Option<Output>, phc_string: String) -> io::Result<(Output, String)> {
    Ok((
        hash.ok_or_else(|| io::Error::other("hasher returned no hash"))?,
        phc_string,
    ))
}

pub(crate) fn derive_password_hash(phsf: &str, password: &[u8]) -> io::Result<Output> {
    let pwhash = PasswordHash::new(phsf).map_err(invalid_data)?;
    let alg = pwhash.algorithm.as_str();
    if alg == argon2::ARGON2D_IDENT.as_str()
        || alg == argon2::ARGON2I_IDENT.as_str()
        || alg == argon2::ARGON2ID_IDENT.as_str()
    {
        let salt = pwhash
            .salt
            .ok_or_else(|| invalid_data("missing salt in password hash"))?;
        let params = argon2::Params::try_from(&pwhash).map_err(invalid_data)?;
        let derived = Argon2::default()
            .hash_password_customized(
                password,
                Some(pwhash.algorithm),
                pwhash.version,
                params,
                salt,
            )
            .map_err(invalid_data)?;
        derived
            .hash
            .ok_or_else(|| io::Error::other("argon2 returned no hash"))
    } else if alg == pbkdf2::Algorithm::PBKDF2_SHA256_ID
        || alg == pbkdf2::Algorithm::PBKDF2_SHA512_ID
    {
        derive_pbkdf2(phsf, password)
    } else {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unsupported algorithm {alg:?}"),
        ))
    }
}

fn derive_pbkdf2(phsf: &str, password: &[u8]) -> io::Result<Output> {
    use ph6::CustomizedPasswordHasher;
    let pwhash = ph6::phc::PasswordHash::new(phsf).map_err(invalid_data)?;
    let salt = pwhash
        .salt
        .ok_or_else(|| invalid_data("missing salt in password hash"))?;
    let params = pbkdf2::Params::try_from(&pwhash).map_err(invalid_data)?;
    let derived = pbkdf2::Pbkdf2::default()
        .hash_password_customized(
            password,
            salt.as_ref(),
            Some(pwhash.algorithm.as_str()),
            pwhash.version,
            params,
        )
        .map_err(invalid_data)?;
    let raw = derived
        .hash
        .ok_or_else(|| io::Error::other("pbkdf2 returned no hash"))?;
    Output::new(raw.as_ref()).map_err(invalid_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::random;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn derive_argon2() {
        let salt = random::salt_string();
        let (expected, ps) = argon2_with_salt(
            b"pass",
            argon2::Algorithm::Argon2id,
            None,
            None,
            None,
            32,
            &salt,
        )
        .unwrap();
        let derived = derive_password_hash(&ps, b"pass").unwrap();
        assert_eq!(derived.as_bytes(), expected.as_bytes());
    }

    #[test]
    fn derive_pbkdf2() {
        let salt = random::salt_string();
        let (expected, ps) = pbkdf2_with_salt(
            b"pass",
            pbkdf2::Algorithm::Pbkdf2Sha256,
            pbkdf2::Params::default(),
            &salt,
        )
        .unwrap();
        let derived = derive_password_hash(&ps, b"pass").unwrap();
        assert_eq!(derived.as_bytes(), expected.as_bytes());
    }
}
