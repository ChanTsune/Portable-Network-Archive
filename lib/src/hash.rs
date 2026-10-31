//! Password hashing helpers.
use crate::KdfCostLimits;
use argon2::{Argon2, ParamsBuilder, Version};
use password_hash::{
    CustomizedPasswordHasher, PasswordHasher,
    phc::{Output, PasswordHash, SaltString},
};
use std::io;

#[inline]
fn invalid_data<E: std::fmt::Display>(e: E) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

/// Maximum Argon2 memory cost (in KiB) accepted for a single key derivation:
/// 512 MiB, about 27 times the `argon2` crate default of 19 MiB.
/// This is the peak memory a single derivation allocates.
const MAX_ARGON2_MEMORY_COST: u32 = 512 * 1024;

/// Maximum Argon2 parallelism (lanes) accepted for a single key derivation.
/// The RFC 9106 parameter recommendations use 4 lanes.
const MAX_ARGON2_PARALLELISM_COST: u32 = 16;

/// Charge added per lane and pass to the memory cost in the total
/// work limit below, in KiB block equivalents: a pass processes four
/// segments per lane, and a segment carries a few extra block
/// compressions of fixed cost for address generation and setup,
/// charged here as four per segment. Argon2i generates address
/// blocks for every segment; Argon2id does so only in its first pass
/// and Argon2d never does, but both are charged the same so the
/// limit stays a single formula. Address generation also costs
/// about 1/64 of the fill work for long segments (two compressions
/// per 128 blocks), which this flat charge does not model, so the
/// true worst case exceeds the nominal limit by under 2%.
const ARGON2_SEGMENT_WORK: u64 = 16;

/// Maximum total Argon2 work accepted for a single key derivation:
/// time cost multiplied by memory cost plus the segment charge, so
/// about 2 GiB of block work, address generation included.
/// Time cost has no separate limit: total work is what consumes time,
/// and a large time cost paired with a small memory cost is a
/// legitimate trade-off under memory constraints that stays cheap.
/// In capped mode, together with the crate's own parameter
/// validation (memory cost at least 8 and at least eight times the
/// parallelism), which the read path completes before any KDF work,
/// this also bounds the time cost to at most 87,381.
const MAX_ARGON2_MEMORY_TIME_COST: u64 = 2 * 1024 * 1024;

/// Maximum PBKDF2 rounds accepted for a single key derivation, as a
/// multiple of the rounds the `pbkdf2` crate recommends for the
/// algorithm (600,000 for SHA-256 and 210,000 for SHA-512).
const MAX_PBKDF2_ROUNDS_FACTOR: u32 = 10;

fn check_argon2_costs(
    time_cost: u32,
    memory_cost: u32,
    parallelism_cost: u32,
) -> Result<(), String> {
    if memory_cost > MAX_ARGON2_MEMORY_COST {
        return Err(format!(
            "argon2 memory cost {memory_cost} exceeds maximum {MAX_ARGON2_MEMORY_COST}"
        ));
    }
    if parallelism_cost > MAX_ARGON2_PARALLELISM_COST {
        return Err(format!(
            "argon2 parallelism cost {parallelism_cost} exceeds maximum {MAX_ARGON2_PARALLELISM_COST}"
        ));
    }
    let total_work = u64::from(time_cost)
        * (u64::from(memory_cost) + ARGON2_SEGMENT_WORK * u64::from(parallelism_cost));
    if total_work > MAX_ARGON2_MEMORY_TIME_COST {
        return Err(format!(
            "argon2 total work from time cost {time_cost}, memory cost {memory_cost} and parallelism cost {parallelism_cost} exceeds maximum {MAX_ARGON2_MEMORY_TIME_COST}"
        ));
    }
    Ok(())
}

fn check_pbkdf2_rounds(algorithm: pbkdf2::Algorithm, rounds: u32) -> Result<(), String> {
    let max_rounds = pbkdf2::Params::recommended_for(algorithm).rounds() * MAX_PBKDF2_ROUNDS_FACTOR;
    if rounds > max_rounds {
        return Err(format!(
            "pbkdf2 rounds {rounds} exceed maximum {max_rounds}"
        ));
    }
    Ok(())
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
    let pwhash = argon2
        .hash_password_with_salt(password, salt.as_ref().as_bytes())
        .map_err(invalid_data)?;
    finalize(pwhash.hash, pwhash.to_string())
}

pub(crate) fn pbkdf2_with_salt(
    password: &[u8],
    algorithm: pbkdf2::Algorithm,
    params: pbkdf2::Params,
    salt: &SaltString,
) -> io::Result<(Output, String)> {
    let pwhash = pbkdf2::Pbkdf2::default()
        .hash_password_customized(
            password,
            salt.as_ref().as_bytes(),
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

pub(crate) fn derive_password_hash(
    phsf: &str,
    password: &[u8],
    limits: KdfCostLimits,
) -> io::Result<Output> {
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
        if !limits.is_unbounded() {
            check_argon2_costs(params.t_cost(), params.m_cost(), params.p_cost())
                .map_err(invalid_data)?;
        }
        let derived = Argon2::default()
            .hash_password_customized(
                password,
                salt.as_ref(),
                Some(pwhash.algorithm.as_str()),
                pwhash.version,
                params,
            )
            .map_err(invalid_data)?;
        derived
            .hash
            .ok_or_else(|| io::Error::other("argon2 returned no hash"))
    } else if alg == pbkdf2::Algorithm::PBKDF2_SHA256_ID {
        derive_pbkdf2(phsf, password, pbkdf2::Algorithm::Pbkdf2Sha256, limits)
    } else if alg == pbkdf2::Algorithm::PBKDF2_SHA512_ID {
        derive_pbkdf2(phsf, password, pbkdf2::Algorithm::Pbkdf2Sha512, limits)
    } else {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unsupported algorithm {alg:?}"),
        ))
    }
}

fn derive_pbkdf2(
    phsf: &str,
    password: &[u8],
    algorithm: pbkdf2::Algorithm,
    limits: KdfCostLimits,
) -> io::Result<Output> {
    let pwhash = PasswordHash::new(phsf).map_err(invalid_data)?;
    let salt = pwhash
        .salt
        .ok_or_else(|| invalid_data("missing salt in password hash"))?;
    let params = pbkdf2::Params::try_from(&pwhash).map_err(invalid_data)?;
    if !limits.is_unbounded() {
        check_pbkdf2_rounds(algorithm, params.rounds()).map_err(invalid_data)?;
    }
    let derived = pbkdf2::Pbkdf2::default()
        .hash_password_customized(
            password,
            salt.as_ref(),
            Some(algorithm.as_ref()),
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
        let salt = random::salt_string().unwrap();
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
        let derived = derive_password_hash(&ps, b"pass", KdfCostLimits::default()).unwrap();
        assert_eq!(derived.as_bytes(), expected.as_bytes());
    }

    #[test]
    fn derive_pbkdf2() {
        let salt = random::salt_string().unwrap();
        let (expected, ps) = pbkdf2_with_salt(
            b"pass",
            pbkdf2::Algorithm::Pbkdf2Sha256,
            pbkdf2::Params::default(),
            &salt,
        )
        .unwrap();
        let derived = derive_password_hash(&ps, b"pass", KdfCostLimits::default()).unwrap();
        assert_eq!(derived.as_bytes(), expected.as_bytes());
    }

    #[test]
    fn argon2_costs_within_limits() {
        for (time_cost, memory_cost, parallelism_cost) in [
            (2, 19456, 1),
            (3, 65536, 4),
            (100, 250, 2),
            (4, 524272, 1),
            (87381, 8, 1),
            (1, 128, 16),
        ] {
            assert!(
                check_argon2_costs(time_cost, memory_cost, parallelism_cost).is_ok(),
                "t={time_cost} m={memory_cost} p={parallelism_cost}"
            );
        }
    }

    #[test]
    fn argon2_costs_over_limits() {
        for (time_cost, memory_cost, parallelism_cost) in
            [(87382, 8, 1), (1, 524289, 1), (1, 136, 17), (5, 524288, 1)]
        {
            assert!(
                check_argon2_costs(time_cost, memory_cost, parallelism_cost).is_err(),
                "t={time_cost} m={memory_cost} p={parallelism_cost}"
            );
        }
    }

    #[test]
    fn pbkdf2_rounds_within_and_over_limits() {
        assert!(check_pbkdf2_rounds(pbkdf2::Algorithm::Pbkdf2Sha256, 6_000_000).is_ok());
        assert!(check_pbkdf2_rounds(pbkdf2::Algorithm::Pbkdf2Sha256, 6_000_001).is_err());
        assert!(check_pbkdf2_rounds(pbkdf2::Algorithm::Pbkdf2Sha512, 2_100_000).is_ok());
        assert!(check_pbkdf2_rounds(pbkdf2::Algorithm::Pbkdf2Sha512, 2_100_001).is_err());
    }

    #[test]
    fn derive_argon2_with_excessive_costs() {
        let salt = random::salt_string().unwrap();
        let (_, ps) = argon2_with_salt(
            b"pass",
            argon2::Algorithm::Argon2id,
            None,
            None,
            None,
            32,
            &salt,
        )
        .unwrap();
        let crafted = ps.replace("m=19456,t=2,p=1", "m=136,t=1,p=17");
        assert_ne!(crafted, ps);
        let Err(err) = derive_password_hash(&crafted, b"pass", KdfCostLimits::default()) else {
            panic!("derivation over the limit must fail");
        };
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn derive_argon2_with_excessive_memory_cost() {
        let salt = random::salt_string().unwrap();
        let (_, ps) = argon2_with_salt(
            b"pass",
            argon2::Algorithm::Argon2id,
            None,
            None,
            None,
            32,
            &salt,
        )
        .unwrap();
        let crafted = ps.replace("m=19456,t=2,p=1", "m=524289,t=1,p=1");
        assert_ne!(crafted, ps);
        let Err(err) = derive_password_hash(&crafted, b"pass", KdfCostLimits::default()) else {
            panic!("derivation over the limit must fail");
        };
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn derive_pbkdf2_with_excessive_rounds() {
        let salt = random::salt_string().unwrap();
        let params = pbkdf2::Params::new_with_output_len(1000, 32).unwrap();
        let (_, ps) =
            pbkdf2_with_salt(b"pass", pbkdf2::Algorithm::Pbkdf2Sha256, params, &salt).unwrap();
        let crafted = ps.replace("i=1000,", "i=6000001,");
        assert_ne!(crafted, ps);
        let Err(err) = derive_password_hash(&crafted, b"pass", KdfCostLimits::default()) else {
            panic!("derivation over the limit must fail");
        };
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn derive_pbkdf2_sha512() {
        let salt = random::salt_string().unwrap();
        let params = pbkdf2::Params::new_with_output_len(1000, 32).unwrap();
        let (expected, ps) =
            pbkdf2_with_salt(b"pass", pbkdf2::Algorithm::Pbkdf2Sha512, params, &salt).unwrap();
        let derived = derive_password_hash(&ps, b"pass", KdfCostLimits::default()).unwrap();
        assert_eq!(derived.as_bytes(), expected.as_bytes());
    }

    #[test]
    fn derive_pbkdf2_sha512_with_excessive_rounds() {
        let salt = random::salt_string().unwrap();
        let params = pbkdf2::Params::new_with_output_len(1000, 32).unwrap();
        let (_, ps) =
            pbkdf2_with_salt(b"pass", pbkdf2::Algorithm::Pbkdf2Sha512, params, &salt).unwrap();
        let crafted = ps.replace("i=1000,", "i=2100001,");
        assert_ne!(crafted, ps);
        let Err(err) = derive_password_hash(&crafted, b"pass", KdfCostLimits::default()) else {
            panic!("derivation over the limit must fail");
        };
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn derive_argon2_with_unbounded_limits_accepts_excessive_costs() {
        let salt = random::salt_string().unwrap();
        let (expected, ps) = argon2_with_salt(
            b"pass",
            argon2::Algorithm::Argon2id,
            Some(1),
            Some(136),
            Some(17),
            32,
            &salt,
        )
        .unwrap();
        let derived = derive_password_hash(&ps, b"pass", KdfCostLimits::unbounded()).unwrap();
        assert_eq!(derived.as_bytes(), expected.as_bytes());
    }
}
