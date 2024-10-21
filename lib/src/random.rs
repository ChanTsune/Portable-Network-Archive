use password_hash::SaltString;
use rand::prelude::*;
use rand_chacha::ChaCha20Rng;
use std::io;

pub(crate) fn random_bytes(dist: &mut [u8]) -> io::Result<()> {
    let mut rand = ChaCha20Rng::from_entropy();
    rand.try_fill_bytes(dist).map_err(io::Error::other)
}

pub(crate) fn random_vec(size: usize) -> io::Result<Vec<u8>> {
    let mut v = vec![0; size];
    random_bytes(&mut v)?;
    Ok(v)
}

pub(crate) fn salt_string() -> SaltString {
    SaltString::generate(ChaCha20Rng::from_entropy())
}
