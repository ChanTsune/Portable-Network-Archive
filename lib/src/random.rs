use password_hash::{Salt, SaltString};
use rand::{TryRngCore, prelude::*};
use rand_chacha::ChaCha20Rng;
use std::io;

pub(crate) fn random_bytes(dist: &mut [u8]) -> io::Result<()> {
    let mut rand = ChaCha20Rng::try_from_os_rng().map_err(io::Error::other)?;
    rand.try_fill_bytes(dist).map_err(io::Error::other)
}

pub(crate) fn random_vec(size: usize) -> io::Result<Vec<u8>> {
    let mut v = vec![0; size];
    random_bytes(&mut v)?;
    Ok(v)
}

pub(crate) fn salt_string() -> io::Result<SaltString> {
    let mut bytes = [0u8; Salt::RECOMMENDED_LENGTH];
    random_bytes(&mut bytes)?;
    SaltString::encode_b64(&bytes).map_err(io::Error::other)
}
