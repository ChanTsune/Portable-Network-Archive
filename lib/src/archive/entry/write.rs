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

fn encryption_writer<W: Write>(
    mut writer: W,
    algorithm: Encryption,
    mode: CipherMode,
    key: &[u8],
    iv: &[u8],
) -> io::Result<CipherWriter<W>> {
    Ok(match (algorithm, mode) {
        (Encryption::No, _) => CipherWriter::No(writer),
        (Encryption::Aes, CipherMode::CBC) => {
            CipherWriter::CbcAes(EncryptCbcAes256Writer::new_with_iv(writer, key, iv)?)
        }
        (Encryption::Aes, CipherMode::CTR) => {
            writer.write_all(iv)?;
            CipherWriter::CtrAes(Ctr128BEWriter::new(writer, key, iv)?)
        }
        (Encryption::Camellia, CipherMode::CBC) => {
            CipherWriter::CbcCamellia(EncryptCbcCamellia256Writer::new_with_iv(writer, key, iv)?)
        }
        (Encryption::Camellia, CipherMode::CTR) => {
            writer.write_all(iv)?;
            CipherWriter::CtrCamellia(Ctr128BEWriter::new(writer, key, iv)?)
        }
    })
}

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

pub(super) fn writer_and_hash<W: Write>(
    writer: W,
    options: WriteOption,
) -> io::Result<(CompressionWriter<CipherWriter<W>>, Option<String>)> {
    let (writer, phsf) = match options.encryption {
        algorithm @ Encryption::No => (
            encryption_writer(writer, algorithm, options.cipher_mode, &[], &[])?,
            None,
        ),
        algorithm @ Encryption::Aes => {
            let salt = random::salt_string();
            let (hash, phsf) = hash(
                algorithm,
                options.hash_algorithm,
                options.password.as_ref().unwrap(),
                &salt,
            )?;
            let iv = random::random_vec(Aes256::block_size())?;
            (
                encryption_writer(writer, algorithm, options.cipher_mode, hash.as_bytes(), &iv)?,
                Some(phsf),
            )
        }
        algorithm @ Encryption::Camellia => {
            let salt = random::salt_string();
            let (hash, phsf) = hash(
                algorithm,
                options.hash_algorithm,
                options.password.as_ref().unwrap(),
                &salt,
            )?;
            let iv = random::random_vec(Camellia256::block_size())?;
            (
                encryption_writer(writer, algorithm, options.cipher_mode, hash.as_bytes(), &iv)?,
                Some(phsf),
            )
        }
    };
    let writer = compression_writer(writer, options.compression, options.compression_level)?;
    Ok((writer, phsf))
}
