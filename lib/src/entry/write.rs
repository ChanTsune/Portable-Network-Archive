use crate::{
    cipher::{CipherWriter, Ctr128BEWriter, EncryptCbcAes256Writer, EncryptCbcCamellia256Writer},
    compress::CompressionWriter,
    entry::{CipherMode, Compression, CompressionLevel, HashAlgorithm, WriteOptions},
    hash, random, Cipher, CipherAlgorithm,
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
    pub(crate) key: Output,
    pub(crate) mode: CipherMode,
}

pub(crate) struct WriteCipher {
    pub(crate) algorithm: CipherAlgorithm,
    pub(crate) context: CipherContext,
}

pub(crate) struct EntryWriterContext {
    pub(crate) compression_level: CompressionLevel,
    pub(crate) compression: Compression,
    pub(crate) cipher: Option<WriteCipher>,
}

#[inline]
fn to_hashed(cipher: Cipher) -> io::Result<WriteCipher> {
    let salt = random::salt_string();
    let (key, phsf) = hash(
        cipher.cipher_algorithm,
        cipher.hash_algorithm,
        cipher.password.as_bytes(),
        &salt,
    )?;
    let iv = match cipher.cipher_algorithm {
        CipherAlgorithm::Aes => random::random_vec(Aes256::block_size()),
        CipherAlgorithm::Camellia => random::random_vec(Camellia256::block_size()),
    }?;
    Ok(WriteCipher {
        algorithm: cipher.cipher_algorithm,
        context: CipherContext {
            phsf,
            iv,
            key,
            mode: cipher.mode,
        },
    })
}

#[inline]
fn get_cipher(cipher: Option<Cipher>) -> io::Result<Option<WriteCipher>> {
    Ok(match cipher {
        None => None,
        Some(it) => Some(to_hashed(it)?),
    })
}

#[inline]
pub(crate) fn get_writer_context(option: WriteOptions) -> io::Result<EntryWriterContext> {
    let cipher = get_cipher(option.cipher)?;
    Ok(EntryWriterContext {
        compression_level: option.compression_level,
        compression: option.compression,
        cipher,
    })
}

#[inline]
fn hash<'s, 'p: 's>(
    cipher_algorithm: CipherAlgorithm,
    hash_algorithm: HashAlgorithm,
    password: &'p [u8],
    salt: &'s SaltString,
) -> io::Result<(Output, String)> {
    #[allow(deprecated)]
    let mut password_hash = match (hash_algorithm, cipher_algorithm) {
        (HashAlgorithm::Argon2Id, CipherAlgorithm::Aes) => hash::argon2_with_salt(
            password,
            argon2::Algorithm::Argon2id,
            Aes256::key_size(),
            salt,
        ),
        (HashAlgorithm::Argon2Id, CipherAlgorithm::Camellia) => hash::argon2_with_salt(
            password,
            argon2::Algorithm::Argon2id,
            Camellia256::key_size(),
            salt,
        ),
        (HashAlgorithm::Pbkdf2Sha256, CipherAlgorithm::Aes | CipherAlgorithm::Camellia) => {
            hash::pbkdf2_with_salt(
                password,
                pbkdf2::Algorithm::Pbkdf2Sha256,
                pbkdf2::Params::default(),
                salt,
            )
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
        Some(WriteCipher {
            algorithm: CipherAlgorithm::Aes,
            context:
                CipherContext {
                    iv,
                    key,
                    mode: CipherMode::CBC,
                    ..
                },
        }) => CipherWriter::CbcAes(EncryptCbcAes256Writer::new(writer, key.as_bytes(), iv)?),
        Some(WriteCipher {
            algorithm: CipherAlgorithm::Aes,
            context:
                CipherContext {
                    iv,
                    key,
                    mode: CipherMode::CTR,
                    ..
                },
        }) => CipherWriter::CtrAes(Ctr128BEWriter::new(writer, key.as_bytes(), iv)?),
        Some(WriteCipher {
            algorithm: CipherAlgorithm::Camellia,
            context:
                CipherContext {
                    iv,
                    key,
                    mode: CipherMode::CBC,
                    ..
                },
        }) => CipherWriter::CbcCamellia(EncryptCbcCamellia256Writer::new(
            writer,
            key.as_bytes(),
            iv,
        )?),
        Some(WriteCipher {
            algorithm: CipherAlgorithm::Camellia,
            context:
                CipherContext {
                    iv,
                    key,
                    mode: CipherMode::CTR,
                    ..
                },
        }) => CipherWriter::CtrCamellia(Ctr128BEWriter::new(writer, key.as_bytes(), iv)?),
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
