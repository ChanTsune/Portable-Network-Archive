//! Entry decryption and decompression reading.

use crate::{
    CipherMode, Compression, Encryption,
    cipher::{Ctr128BEReader, DecryptCbcAes256Reader, DecryptCbcCamellia256Reader, DecryptReader},
    compress::DecompressReader,
    entry::KeyCache,
    hash::derive_password_hash,
};
use aes::Aes256;
use camellia::Camellia256;
use crypto_common::BlockSizeUser;
use password_hash::Output;
use std::io::{self, Read};

/// Resolves the cipher key for a PHC string, reusing a previously derived
/// key when the cache holds one.
///
/// The KDF runs outside the cache lock: concurrent readers may derive the
/// same key more than once on first contact, but the result is
/// deterministic so the race is benign.
fn resolve_key(phsf: &str, password: &[u8], key_cache: Option<&KeyCache>) -> io::Result<Output> {
    if let Some(cache) = key_cache
        && let Some(key) = cache.get(phsf)
    {
        return Ok(key);
    }
    let password_hash = derive_password_hash(phsf, password)?;
    let key = password_hash
        .hash
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "failed to get hash"))?;
    if let Some(cache) = key_cache {
        cache.insert(phsf, key);
    }
    Ok(key)
}

/// Decrypt reader according to an encryption type.
pub(crate) fn decrypt_reader<R: Read>(
    mut reader: R,
    encryption: Encryption,
    cipher_mode: CipherMode,
    phsf: Option<&str>,
    password: Option<&[u8]>,
    key_cache: Option<&KeyCache>,
) -> io::Result<DecryptReader<R>> {
    Ok(match encryption {
        Encryption::No => DecryptReader::No(reader),
        encryption @ (Encryption::Aes | Encryption::Camellia) => {
            let s = phsf.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "`PHSF` chunk not found")
            })?;
            let password = password.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "password was not provided")
            })?;
            let hash = resolve_key(s, password, key_cache)?;
            let key = hash.as_bytes();
            match (encryption, cipher_mode) {
                (Encryption::Aes, CipherMode::CBC) => {
                    let mut iv = vec![0; Aes256::block_size()];
                    reader.read_exact(&mut iv)?;
                    DecryptReader::CbcAes(DecryptCbcAes256Reader::new(reader, key, &iv)?)
                }
                (Encryption::Aes, CipherMode::CTR) => {
                    let mut iv = vec![0u8; Aes256::block_size()];
                    reader.read_exact(&mut iv)?;
                    DecryptReader::CtrAes(Ctr128BEReader::new(reader, key, &iv)?)
                }
                (Encryption::Camellia, CipherMode::CBC) => {
                    let mut iv = vec![0; Camellia256::block_size()];
                    reader.read_exact(&mut iv)?;
                    DecryptReader::CbcCamellia(DecryptCbcCamellia256Reader::new(reader, key, &iv)?)
                }
                (Encryption::Camellia, CipherMode::CTR) => {
                    let mut iv = vec![0u8; Camellia256::block_size()];
                    reader.read_exact(&mut iv)?;
                    DecryptReader::CtrCamellia(Ctr128BEReader::new(reader, key, &iv)?)
                }
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!("unsupported cipher mode: {cipher_mode:?}"),
                    ));
                }
            }
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("unsupported encryption method: {encryption:?}"),
            ));
        }
    })
}

const DECOMPRESS_BUFFER_SIZE: usize = 32 * 1024;

/// Decompress reader according to a compression type.
pub(crate) fn decompress_reader<R: Read>(
    reader: R,
    compression: Compression,
) -> io::Result<DecompressReader<R>> {
    let reader = io::BufReader::with_capacity(DECOMPRESS_BUFFER_SIZE, reader);
    Ok(match compression {
        Compression::No => DecompressReader::No(reader),
        Compression::Deflate => {
            DecompressReader::Deflate(flate2::bufread::ZlibDecoder::new(reader))
        }
        Compression::ZStandard => DecompressReader::ZStd(zstd::Decoder::with_buffer(reader)?),
        Compression::XZ => DecompressReader::Xz(liblzma::bufread::XzDecoder::new(reader)),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("unsupported compression method: {compression:?}"),
            ));
        }
    })
}

pub(crate) struct EntryReader<R: Read>(pub(crate) DecompressReader<DecryptReader<R>>);

impl<R: Read> Read for EntryReader<R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    use wasm_bindgen_test::wasm_bindgen_test as test;

    #[test]
    fn decrypt_reader_rejects_unknown_cipher_mode() {
        let phsf = "$argon2id$v=19$m=8,t=1,p=1$c29tZXNhbHQ";
        let result = decrypt_reader(
            io::Cursor::new(Vec::<u8>::new()),
            Encryption::Camellia,
            CipherMode::Reserved(3),
            Some(phsf),
            Some(b"password"),
            None,
        );
        assert!(matches!(
            result,
            Err(e) if e.kind() == io::ErrorKind::Unsupported
        ));
    }
}
