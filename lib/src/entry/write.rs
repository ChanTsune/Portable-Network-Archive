//! Entry compression and encryption writing.

use crate::{
    ChunkType, Cipher, CipherAlgorithm, HashAlgorithm,
    cipher::{
        CipherWriter, Ctr128BEWriter, EncryptCbcAes256Writer, EncryptCbcCamellia256Writer,
        EncryptGcmAes256Writer, EncryptGcmCamellia256Writer, StreamHeader, derive_stream_key,
    },
    compress::CompressionWriter,
    entry::{CipherMode, Compress, DerivedKeyMaterial, HashAlgorithmParams, WriteOption},
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
    pub(crate) payload: CipherPayload,
}

pub(crate) enum CipherPayload {
    /// CBC/CTR: the KDF output is used directly as the encryption key.
    Block {
        mode: CipherMode,
        iv: Vec<u8>,
        key: Output,
    },
    /// GCM: the per-stream key is derived from the KDF output (K_master) and
    /// the stream header.
    GcmStream {
        header: StreamHeader,
        k_stream: [u8; 32],
    },
}

impl CipherContext {
    /// On-wire datastream prefix: the block IV for CBC/CTR, or the stream
    /// header for GCM.
    pub(crate) fn prefix_bytes(&self) -> Vec<u8> {
        match &self.payload {
            CipherPayload::Block { iv, .. } => iv.clone(),
            CipherPayload::GcmStream { header, .. } => header.to_bytes().to_vec(),
        }
    }
}

pub(crate) struct WriteCipher {
    pub(crate) algorithm: CipherAlgorithm,
    pub(crate) context: CipherContext,
}

pub(crate) struct EntryWriterContext {
    pub(crate) compress: Compress,
    pub(crate) cipher: Option<WriteCipher>,
}

pub(crate) fn derive_key_material(
    cipher_algorithm: CipherAlgorithm,
    hash_algorithm: HashAlgorithm,
    password: &[u8],
) -> io::Result<DerivedKeyMaterial> {
    let salt = random::salt_string();
    let (key, phsf) = hash(cipher_algorithm, hash_algorithm, password, &salt)?;
    Ok(DerivedKeyMaterial { phsf, key })
}

#[inline]
fn to_hashed(
    cipher: &Cipher,
    header_chunk_type: ChunkType,
    header_chunk_data: &[u8],
) -> io::Result<WriteCipher> {
    let context = match cipher.mode {
        CipherMode::GCM => {
            let mut salt = [0u8; 32];
            random::random_bytes(&mut salt)?;
            let mut nonce_prefix = [0u8; 7];
            random::random_bytes(&mut nonce_prefix)?;
            let header = StreamHeader::new(salt, nonce_prefix, cipher.segment_size)
                .map_err(io::Error::other)?;
            let k_stream = derive_stream_key(
                cipher.derived.key.as_bytes(),
                &header.salt,
                header_chunk_type,
                header_chunk_data,
                cipher.derived.phsf.as_bytes(),
            );
            CipherContext {
                phsf: cipher.derived.phsf.clone(),
                payload: CipherPayload::GcmStream { header, k_stream },
            }
        }
        _ => {
            let iv = match cipher.cipher_algorithm {
                CipherAlgorithm::Aes => random::random_vec(Aes256::block_size()),
                CipherAlgorithm::Camellia => random::random_vec(Camellia256::block_size()),
            }?;
            CipherContext {
                phsf: cipher.derived.phsf.clone(),
                payload: CipherPayload::Block {
                    mode: cipher.mode,
                    iv,
                    key: cipher.derived.key,
                },
            }
        }
    };
    Ok(WriteCipher {
        algorithm: cipher.cipher_algorithm,
        context,
    })
}

#[inline]
pub(crate) fn get_writer_context(
    option: impl WriteOption,
    header_chunk_type: ChunkType,
    header_chunk_data: &[u8],
) -> io::Result<EntryWriterContext> {
    let cipher = option
        .cipher()
        .map(|c| to_hashed(c, header_chunk_type, header_chunk_data))
        .transpose()?;
    Ok(EntryWriterContext {
        compress: option.compress(),
        cipher,
    })
}

#[inline]
fn key_size(cipher_algorithm: CipherAlgorithm) -> usize {
    match cipher_algorithm {
        CipherAlgorithm::Aes => Aes256::key_size(),
        CipherAlgorithm::Camellia => Camellia256::key_size(),
    }
}

#[inline]
fn hash<'s, 'p: 's>(
    cipher_algorithm: CipherAlgorithm,
    hash_algorithm: HashAlgorithm,
    password: &'p [u8],
    salt: &'s SaltString,
) -> io::Result<(Output, String)> {
    let mut password_hash = match hash_algorithm.0 {
        HashAlgorithmParams::Argon2Id {
            time_cost,
            memory_cost,
            parallelism_cost,
        } => hash::argon2_with_salt(
            password,
            argon2::Algorithm::Argon2id,
            time_cost,
            memory_cost,
            parallelism_cost,
            key_size(cipher_algorithm),
            salt,
        ),
        HashAlgorithmParams::Pbkdf2Sha256 { rounds } => {
            let mut params = pbkdf2::Params::default();
            if let Some(rounds) = rounds {
                params.rounds = rounds;
            }
            hash::pbkdf2_with_salt(password, pbkdf2::Algorithm::Pbkdf2Sha256, params, salt)
        }
    }?;
    let hash = password_hash
        .hash
        .take()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "failed to get hash"))?;
    Ok((hash, password_hash.to_string()))
}

#[inline]
fn encryption_writer<W: Write>(
    writer: W,
    cipher: &Option<WriteCipher>,
) -> io::Result<CipherWriter<W>> {
    Ok(match cipher {
        None => CipherWriter::No(writer),
        Some(WriteCipher { algorithm, context }) => match (algorithm, &context.payload) {
            (
                CipherAlgorithm::Aes,
                CipherPayload::Block {
                    mode: CipherMode::CBC,
                    iv,
                    key,
                },
            ) => CipherWriter::CbcAes(EncryptCbcAes256Writer::new(writer, key.as_bytes(), iv)?),
            (
                CipherAlgorithm::Aes,
                CipherPayload::Block {
                    mode: CipherMode::CTR,
                    iv,
                    key,
                },
            ) => CipherWriter::CtrAes(Ctr128BEWriter::new(writer, key.as_bytes(), iv)?),
            (
                CipherAlgorithm::Camellia,
                CipherPayload::Block {
                    mode: CipherMode::CBC,
                    iv,
                    key,
                },
            ) => CipherWriter::CbcCamellia(EncryptCbcCamellia256Writer::new(
                writer,
                key.as_bytes(),
                iv,
            )?),
            (
                CipherAlgorithm::Camellia,
                CipherPayload::Block {
                    mode: CipherMode::CTR,
                    iv,
                    key,
                },
            ) => CipherWriter::CtrCamellia(Ctr128BEWriter::new(writer, key.as_bytes(), iv)?),
            (CipherAlgorithm::Aes, CipherPayload::GcmStream { header, k_stream }) => {
                CipherWriter::GcmAes(EncryptGcmAes256Writer::new(writer, k_stream, header))
            }
            (CipherAlgorithm::Camellia, CipherPayload::GcmStream { header, k_stream }) => {
                CipherWriter::GcmCamellia(EncryptGcmCamellia256Writer::new(
                    writer, k_stream, header,
                ))
            }
            (_, CipherPayload::Block { mode, .. }) => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!("unsupported cipher mode for writing: {mode:?}"),
                ));
            }
        },
    })
}

#[inline]
fn compression_writer<W: Write>(
    writer: W,
    algorithm: Compress,
) -> io::Result<CompressionWriter<W>> {
    Ok(match algorithm {
        Compress::No => CompressionWriter::No(writer),
        Compress::Deflate(level) => {
            CompressionWriter::Deflate(ZlibEncoder::new(writer, level.into()))
        }
        Compress::ZStandard(level) => {
            CompressionWriter::ZStd(ZstdEncoder::new(writer, level.into())?)
        }
        Compress::XZ(level) => CompressionWriter::Xz(XzEncoder::new(writer, level.into())),
    })
}

#[inline]
pub(crate) fn get_writer<W: Write>(
    writer: W,
    context: &EntryWriterContext,
) -> io::Result<CompressionWriter<CipherWriter<W>>> {
    let writer = encryption_writer(writer, &context.cipher)?;
    compression_writer(writer, context.compress)
}
