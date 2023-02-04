use password_hash::SaltString;
use rand::{rngs::OsRng, RngCore};
use std::io;

pub fn random_bytes(dist: &mut [u8]) -> io::Result<()> {
    let mut rand = OsRng::default();
    rand.try_fill_bytes(dist)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
}

pub fn salt_string() -> SaltString {
    SaltString::generate(OsRng::default())
}
