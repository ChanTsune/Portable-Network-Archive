use crate::{
    archive::{CipherMode, Compression, CompressionLevel, Encryption, HashAlgorithm, WriteOption},
    cipher::{CipherWriter, Ctr128BEWriter, EncryptCbcAes256Writer, EncryptCbcCamellia256Writer},
    compress::CompressionWriter,
    hash, random,
};
use aes::Aes256;
use camellia::Camellia256;
use crypto_common::{BlockSizeUser, KeySizeUser};
use flate2::write::ZlibEncoder;
use liblzma::write::XzEncoder;
use password_hash::{Output, SaltString};
use std::io::{self, Write};
use zstd::stream::write::Encoder as ZstdEncoder;

pub(crate) struct CipherContext {
    iv: Vec<u8>,
    key: Vec<u8>,
    mode: CipherMode,
}

pub(crate) enum Cipher {
    None,
    Aes(CipherContext),
    Camellia(CipherContext),
}

#[inline]
fn get_cipher(
    password: Option<&str>,
    hash_algorithm: HashAlgorithm,
    algorithm: Encryption,
    mode: CipherMode,
) -> io::Result<(Cipher, Option<String>)> {
    Ok(match algorithm {
        Encryption::No => (Cipher::None, None),
        Encryption::Aes => {
            let salt = random::salt_string();
            let (hash, phsf) = hash(algorithm, hash_algorithm, password.unwrap(), &salt)?;
            let iv = random::random_vec(Aes256::block_size())?;
            (
                Cipher::Aes(CipherContext {
                    iv,
                    key: hash.as_bytes().to_vec(),
                    mode,
                }),
                Some(phsf),
            )
        }
        Encryption::Camellia => {
            let salt = random::salt_string();
            let (hash, phsf) = hash(algorithm, hash_algorithm, password.unwrap(), &salt)?;
            let iv = random::random_vec(Camellia256::block_size())?;
            (
                Cipher::Camellia(CipherContext {
                    iv,
                    key: hash.as_bytes().to_vec(),
                    mode,
                }),
                Some(phsf),
            )
        }
    })
}

#[inline]
fn hash<'s, 'p: 's>(
    encryption_algorithm: Encryption,
    hash_algorithm: HashAlgorithm,
    password: &'p str,
    salt: &'s SaltString,
) -> io::Result<(Output, String)> {
    let mut password_hash = match (hash_algorithm, encryption_algorithm) {
        (HashAlgorithm::Argon2Id, Encryption::Aes) => hash::argon2_with_salt(
            password,
            argon2::Algorithm::Argon2id,
            Aes256::key_size(),
            salt,
        ),
        (HashAlgorithm::Argon2Id, Encryption::Camellia) => hash::argon2_with_salt(
            password,
            argon2::Algorithm::Argon2id,
            Camellia256::key_size(),
            salt,
        ),
        (HashAlgorithm::Pbkdf2Sha256, Encryption::Aes | Encryption::Camellia) => {
            hash::pbkdf2_with_salt(
                password,
                pbkdf2::Algorithm::Pbkdf2Sha256,
                pbkdf2::Params::default(),
                salt,
            )
        }
        (_, Encryption::No) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid combination",
            ))
        }
    }?;
    let hash = password_hash
        .hash
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "Failed to get hash"))?;
    Ok((hash, password_hash.to_string()))
}

#[inline]
fn encryption_writer<W: Write>(writer: W, cipher: &Cipher) -> io::Result<CipherWriter<W>> {
    Ok(match cipher {
        Cipher::None => CipherWriter::No(writer),
        Cipher::Aes(CipherContext {
            iv,
            key,
            mode: CipherMode::CBC,
        }) => CipherWriter::CbcAes(EncryptCbcAes256Writer::new(writer, key, iv)?),
        Cipher::Aes(CipherContext {
            iv,
            key,
            mode: CipherMode::CTR,
        }) => CipherWriter::CtrAes(Ctr128BEWriter::new(writer, key, iv)?),
        Cipher::Camellia(CipherContext {
            iv,
            key,
            mode: CipherMode::CBC,
        }) => CipherWriter::CbcCamellia(EncryptCbcCamellia256Writer::new(writer, key, iv)?),
        Cipher::Camellia(CipherContext {
            iv,
            key,
            mode: CipherMode::CTR,
        }) => CipherWriter::CtrCamellia(Ctr128BEWriter::new(writer, key, iv)?),
    })
}

#[inline]
fn compression_writer<W: Write>(
    writer: W,
    algorithm: Compression,
    level: CompressionLevel,
) -> io::Result<CompressionWriter<W>> {
    Ok(match algorithm {
        Compression::No => CompressionWriter::No(writer),
        Compression::Deflate => CompressionWriter::Deflate(ZlibEncoder::new(writer, level.into())),
        Compression::ZStandard => CompressionWriter::ZStd(ZstdEncoder::new(writer, level.into())?),
        Compression::XZ => CompressionWriter::Xz(XzEncoder::new(writer, level.into())),
    })
}

#[inline]
pub(super) fn writer_and_hash<W: Write>(
    writer: W,
    options: WriteOption,
) -> io::Result<(
    CompressionWriter<CipherWriter<W>>,
    Option<Vec<u8>>,
    Option<String>,
)> {
    let (cipher, phsf) = get_cipher(
        options.password.as_deref(),
        options.hash_algorithm,
        options.encryption,
        options.cipher_mode,
    )?;
    let writer = encryption_writer(writer, &cipher)?;
    let writer = compression_writer(writer, options.compression, options.compression_level)?;
    Ok((
        writer,
        match cipher {
            Cipher::None => None,
            Cipher::Aes(c) | Cipher::Camellia(c) => Some(c.iv),
        },
        phsf,
    ))
}
