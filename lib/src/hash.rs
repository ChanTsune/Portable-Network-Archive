//! Password hashing helpers.
use argon2::{Argon2, ParamsBuilder, Version};
use password_hash::{Encoding::B64, Output, PasswordHash, PasswordHasher, SaltString};
use std::{fmt, io, str::FromStr};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub(crate) struct PHCStringWithVerifier {
    phc_string: String,
    verifier: Option<Vec<u8>>,
}

impl PHCStringWithVerifier {
    #[inline]
    fn new(password_hash: &PasswordHash<'_>, output: &Output) -> Self {
        Self {
            phc_string: password_hash.to_string(),
            verifier: Some(output.as_bytes().last_chunk::<2>().expect("").to_vec()),
        }
    }
}

impl fmt::Display for PHCStringWithVerifier {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.phc_string)?;
        if let Some(v) = &self.verifier {
            let mut buffer = [b'\0'; 32];
            write!(
                f,
                "${}",
                B64.encode(v, &mut buffer).map_err(|_| fmt::Error)?
            )?;
        };
        Ok(())
    }
}

impl FromStr for PHCStringWithVerifier {
    type Err = String;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut fields = s.split('$');
        let _begin = fields.next();
        let _algorithm = fields.next();
        let maybe_version = fields.next();
        let separator_count =
            if maybe_version.is_some_and(|it| it.starts_with("v=") && !it.contains(',')) {
                5
            } else {
                4
            };
        if s.chars().filter(|&c| c == '$').count() == separator_count {
            let Some((phc_string, verifier)) = s.rsplit_once('$') else {
                return Err(format!("Invalid hash string: {}", s));
            };
            let mut buffer = [b'\0'; 32];
            Ok(Self {
                phc_string: phc_string.to_string(),
                verifier: Some(
                    B64.decode(verifier, &mut buffer)
                        .map_err(|e| format!("{e}"))?
                        .to_vec(),
                ),
            })
        } else {
            Ok(Self {
                phc_string: s.to_string(),
                verifier: None,
            })
        }
    }
}

pub(crate) fn argon2_with_salt<'a>(
    password: &'a [u8],
    algorithm: argon2::Algorithm,
    time_cost: Option<u32>,
    memory_cost: Option<u32>,
    parallelism_cost: Option<u32>,
    hash_length: usize,
    salt: &'a SaltString,
) -> io::Result<(PHCStringWithVerifier, Output)> {
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
    let mut ph = argon2
        .hash_password(password, salt)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let output = ph
        .hash
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "failed to get hash"))?;
    Ok((PHCStringWithVerifier::new(&ph, &output), output))
}

pub(crate) fn pbkdf2_with_salt<'a>(
    password: &'a [u8],
    algorithm: pbkdf2::Algorithm,
    params: pbkdf2::Params,
    salt: &'a SaltString,
) -> io::Result<(PHCStringWithVerifier, Output)> {
    let mut ph = pbkdf2::Pbkdf2
        .hash_password_customized(password, Some(algorithm.ident()), None, params, salt)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let output = ph
        .hash
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "failed to get hash"))?;
    Ok((PHCStringWithVerifier::new(&ph, &output), output))
}

pub(crate) fn derive_password_hash(
    phsf: &PHCStringWithVerifier,
    password: &[u8],
) -> io::Result<Output> {
    let password_hash = PasswordHash::new(&phsf.phc_string)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let salt = password_hash.salt.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing salt in password hash")
    })?;
    let password_hash = match password_hash.algorithm {
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
    }?;
    password_hash
        .hash
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "failed to get hash"))
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
        let (phs_original, output) = argon2_with_salt(
            b"pass",
            argon2::Algorithm::Argon2id,
            None,
            None,
            None,
            32,
            &salt,
        )
        .unwrap();
        let phsf = phs_original.to_string();
        let phs = PHCStringWithVerifier::from_str(&phsf).unwrap();
        assert_eq!(phs_original, phs);
        let output2 = derive_password_hash(&phs, b"pass").unwrap();
        assert_eq!(output, output2);
    }

    #[test]
    fn derive_pbkdf2() {
        let salt = random::salt_string();
        let (phs_original, output) = pbkdf2_with_salt(
            b"pass",
            pbkdf2::Algorithm::Pbkdf2Sha256,
            pbkdf2::Params::default(),
            &salt,
        )
        .unwrap();
        let phsf = phs_original.to_string();
        let phs = PHCStringWithVerifier::from_str(&phsf).unwrap();
        assert_eq!(phs_original, phs);
        let output2 = derive_password_hash(&phs, b"pass").unwrap();
        assert_eq!(output, output2);
    }
}
