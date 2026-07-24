//! Entry decryption and decompression reading.

use crate::{
    ChunkType, CipherMode, Compression, Encryption,
    cipher::{
        Ctr128BEReader, DecryptCbcAes256Reader, DecryptCbcCamellia256Reader,
        DecryptGcmAes256Reader, DecryptGcmCamellia256Reader, DecryptReader, STREAM_HEADER_LEN,
        StreamHeader, derive_stream_key,
    },
    compress::DecompressReader,
    entry::{KeyCache, ReadOption},
    error::AeadError,
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

/// Recovers the KDF output (`K_master`) from a stored `PHSF` and a password,
/// reusing a previously derived key when the cache holds one.
#[inline]
fn derive_key(
    phsf: &str,
    password: Option<&[u8]>,
    key_cache: Option<&KeyCache>,
) -> io::Result<Output> {
    let password = password
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "password was not provided"))?;
    resolve_key(phsf, password, key_cache)
}

/// Decrypt reader according to an encryption type.
///
/// `options` supplies the password and the derived-key cache. `header_chunk_type`
/// and `header_chunk_data` are the raw `FHED`/`SHED` Type and Data fields as read
/// from the archive; both feed AEAD (GCM) stream-key derivation and are ignored
/// by the block/stream cipher modes.
pub(crate) fn decrypt_reader<R: Read, O: ReadOption>(
    mut reader: R,
    encryption: Encryption,
    cipher_mode: CipherMode,
    phsf: Option<&str>,
    options: O,
    header_chunk_type: ChunkType,
    header_chunk_data: &[u8],
) -> io::Result<DecryptReader<R>> {
    let password = options.password();
    let key_cache = options.key_cache();
    Ok(match encryption {
        Encryption::NO => DecryptReader::No(reader),
        encryption @ (Encryption::AES | Encryption::CAMELLIA) => {
            let s = phsf.ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "`PHSF` chunk not found")
            })?;
            match (encryption, cipher_mode) {
                (Encryption::AES, CipherMode::CBC) => {
                    let key = derive_key(s, password, key_cache)?;
                    let mut iv = vec![0; Aes256::block_size()];
                    reader.read_exact(&mut iv)?;
                    DecryptReader::CbcAes(DecryptCbcAes256Reader::new(reader, key.as_bytes(), &iv)?)
                }
                (Encryption::AES, CipherMode::CTR) => {
                    let key = derive_key(s, password, key_cache)?;
                    let mut iv = vec![0u8; Aes256::block_size()];
                    reader.read_exact(&mut iv)?;
                    DecryptReader::CtrAes(Ctr128BEReader::new(reader, key.as_bytes(), &iv)?)
                }
                (Encryption::CAMELLIA, CipherMode::CBC) => {
                    let key = derive_key(s, password, key_cache)?;
                    let mut iv = vec![0; Camellia256::block_size()];
                    reader.read_exact(&mut iv)?;
                    DecryptReader::CbcCamellia(DecryptCbcCamellia256Reader::new(
                        reader,
                        key.as_bytes(),
                        &iv,
                    )?)
                }
                (Encryption::CAMELLIA, CipherMode::CTR) => {
                    let key = derive_key(s, password, key_cache)?;
                    let mut iv = vec![0u8; Camellia256::block_size()];
                    reader.read_exact(&mut iv)?;
                    DecryptReader::CtrCamellia(Ctr128BEReader::new(reader, key.as_bytes(), &iv)?)
                }
                (enc, CipherMode::GCM) => {
                    let mut header = [0u8; STREAM_HEADER_LEN];
                    reader.read_exact(&mut header).map_err(|e| {
                        if e.kind() == io::ErrorKind::UnexpectedEof {
                            AeadError::Malformed("datastream shorter than the stream header").into()
                        } else {
                            e
                        }
                    })?;
                    // Parsed (and the segment size range-checked) before the
                    // password KDF runs, so a malformed stream fails without
                    // paying the (deliberately expensive) Argon2id cost.
                    let header = StreamHeader::try_from_bytes(&header)?;
                    let k_master = derive_key(s, password, key_cache)?;
                    if k_master.as_bytes().len() != 32 {
                        return Err(AeadError::Malformed("K_master is not 32 bytes").into());
                    }
                    let k_stream = derive_stream_key(
                        k_master.as_bytes(),
                        &header.salt,
                        header_chunk_type,
                        header_chunk_data,
                        s.as_bytes(),
                    );
                    match enc {
                        Encryption::AES => DecryptReader::GcmAes(DecryptGcmAes256Reader::new(
                            reader, &k_stream, &header,
                        )),
                        Encryption::CAMELLIA => DecryptReader::GcmCamellia(
                            DecryptGcmCamellia256Reader::new(reader, &k_stream, &header),
                        ),
                        _ => {
                            return Err(io::Error::new(
                                io::ErrorKind::Unsupported,
                                format!("unsupported encryption algorithm for GCM: {enc:?}"),
                            ));
                        }
                    }
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
        Compression::NO => DecompressReader::No(reader),
        Compression::DEFLATE => {
            DecompressReader::Deflate(flate2::bufread::ZlibDecoder::new(reader))
        }
        Compression::ZSTANDARD => DecompressReader::ZStd(zstd::Decoder::with_buffer(reader)?),
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
            Encryption::CAMELLIA,
            CipherMode::from_byte(3),
            Some(phsf),
            crate::ReadOptions::with_password(Some("password")),
            ChunkType::FHED,
            b"",
        );
        assert!(matches!(
            result,
            Err(e) if e.kind() == io::ErrorKind::Unsupported
        ));
    }
}
