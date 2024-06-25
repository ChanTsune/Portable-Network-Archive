use crate::{
    cipher::{CipherWriter, Ctr128BEWriter, EncryptCbcAes256Writer, EncryptCbcCamellia256Writer},
    compress::CompressionWriter,
    entry::{CipherMode, Compression, CompressionLevel, Encryption, HashAlgorithm, WriteOptions},
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
    pub(crate) phsf: String,
    pub(crate) iv: Vec<u8>,
    pub(crate) key: Vec<u8>,
    pub(crate) mode: CipherMode,
}

pub(crate) enum WriteCipher {
    Aes(CipherContext),
    Camellia(CipherContext),
}

pub(crate) struct EntryWriterContext {
    pub(crate) compression_level: CompressionLevel,
    pub(crate) compression: Compression,
    pub(crate) cipher: Option<WriteCipher>,
}

#[inline]
fn get_cipher(
    password: Option<&str>,
    hash_algorithm: HashAlgorithm,
    algorithm: Encryption,
    mode: CipherMode,
) -> io::Result<Option<WriteCipher>> {
    Ok(match algorithm {
        Encryption::No => None,
        Encryption::Aes => {
            let salt = random::salt_string();
            let (hash, phsf) = hash(algorithm, hash_algorithm, password.unwrap(), &salt)?;
            let iv = random::random_vec(Aes256::block_size())?;
            Some(WriteCipher::Aes(CipherContext {
                phsf,
                iv,
                key: hash.as_bytes().to_vec(),
                mode,
            }))
        }
        Encryption::Camellia => {
            let salt = random::salt_string();
            let (hash, phsf) = hash(algorithm, hash_algorithm, password.unwrap(), &salt)?;
            let iv = random::random_vec(Camellia256::block_size())?;
            Some(WriteCipher::Camellia(CipherContext {
                phsf,
                iv,
                key: hash.as_bytes().to_vec(),
                mode,
            }))
        }
    })
}

#[inline]
pub(crate) fn get_writer_context(option: WriteOptions) -> io::Result<EntryWriterContext> {
    let cipher = get_cipher(
        option.password.as_deref(),
        option.hash_algorithm,
        option.encryption,
        option.cipher_mode,
    )?;
    Ok(EntryWriterContext {
        compression_level: option.compression_level,
        compression: option.compression,
        cipher,
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
fn encryption_writer<W: Write>(
    writer: W,
    cipher: &Option<WriteCipher>,
) -> io::Result<CipherWriter<W>> {
    Ok(match cipher {
        None => CipherWriter::No(writer),
        Some(WriteCipher::Aes(CipherContext {
            iv,
            key,
            mode: CipherMode::CBC,
            ..
        })) => CipherWriter::CbcAes(EncryptCbcAes256Writer::new(writer, key, iv)?),
        Some(WriteCipher::Aes(CipherContext {
            iv,
            key,
            mode: CipherMode::CTR,
            ..
        })) => CipherWriter::CtrAes(Ctr128BEWriter::new(writer, key, iv)?),
        Some(WriteCipher::Camellia(CipherContext {
            iv,
            key,
            mode: CipherMode::CBC,
            ..
        })) => CipherWriter::CbcCamellia(EncryptCbcCamellia256Writer::new(writer, key, iv)?),
        Some(WriteCipher::Camellia(CipherContext {
            iv,
            key,
            mode: CipherMode::CTR,
            ..
        })) => CipherWriter::CtrCamellia(Ctr128BEWriter::new(writer, key, iv)?),
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
pub(crate) fn get_writer<W: Write>(
    writer: W,
    context: &EntryWriterContext,
) -> io::Result<CompressionWriter<CipherWriter<W>>> {
    let writer = encryption_writer(writer, &context.cipher)?;
    compression_writer(writer, context.compression, context.compression_level)
}
