//! Random salt and initialization vector generation.

use password_hash::phc::{Salt, SaltString};
use rand::{TryRng, rngs::SysRng};
use std::io;

pub(crate) fn random_bytes(dist: &mut [u8]) -> io::Result<()> {
    SysRng.try_fill_bytes(dist).map_err(io::Error::other)
}

pub(crate) fn random_vec(size: usize) -> io::Result<Vec<u8>> {
    let mut v = vec![0; size];
    random_bytes(&mut v)?;
    Ok(v)
}

pub(crate) fn salt_string() -> io::Result<SaltString> {
    let mut bytes = [0u8; Salt::RECOMMENDED_LENGTH];
    random_bytes(&mut bytes)?;
    Ok(Salt::new(&bytes)
        .map_err(io::Error::other)?
        .to_salt_string())
}
