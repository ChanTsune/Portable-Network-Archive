use crate::{
    archive::{
        CipherMode, Compression, CompressionLevel, DataKind, Encryption, EntryHeader, EntryName,
        HashAlgorithm, WriteOption,
    },
    chunk::{self, ChunkWriter},
    cipher::{CipherWriter, Ctr128BEWriter, EncryptCbcAes256Writer, EncryptCbcCamellia256Writer},
    compress::CompressionWriter,
    hash,
    io::TryIntoInner,
    random,
};
use aes::Aes256;
use camellia::Camellia256;
use crypto_common::{BlockSizeUser, KeySizeUser};
use flate2::write::DeflateEncoder;
use password_hash::{Output, SaltString};
use std::io::{self, Write};
use xz2::write::XzEncoder;
use zstd::stream::write::Encoder as ZstdEncoder;

pub(crate) struct EntryWriter<W: Write> {
    w: W,
    options: WriteOption,
    buf: Vec<u8>,
}

impl<W: Write> EntryWriter<W> {
    pub(crate) fn new_file_with(w: W, name: EntryName, options: WriteOption) -> io::Result<Self> {
        let mut chunk_writer = ChunkWriter::from(w);
        chunk_writer.write_chunk(
            chunk::FHED,
            &EntryHeader::new(
                DataKind::File,
                options.compression,
                options.encryption,
                options.cipher_mode,
                name,
            )
            .to_bytes(),
        )?;
        Ok(Self {
            w: chunk_writer.into_inner(),
            options,
            buf: Vec::new(),
        })
    }

    pub(crate) fn finish(mut self) -> io::Result<W> {
        let (phsf, data) = {
            let (mut writer, phsf) = writer_and_hash(Vec::new(), &self.options)?;
            writer.write_all(&self.buf)?;
            (phsf, writer.try_into_inner()?.try_into_inner()?)
        };

        let mut chunk_writer = ChunkWriter::from(&mut self.w);

        if let Some(phsf) = phsf {
            chunk_writer.write_chunk(chunk::PHSF, phsf.as_bytes())?;
        }
        chunk_writer.write_chunk(chunk::FDAT, &data)?;

        chunk_writer.write_chunk(chunk::FEND, &[])?;
        Ok(self.w)
    }
}

impl<W: Write> Write for EntryWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn hash<'s, 'p: 's>(
    encryption_algorithm: Encryption,
    hash_algorithm: HashAlgorithm,
    password: &'p str,
    salt: &'s SaltString,
) -> io::Result<(Output, String)> {
    let mut password_hash = match (hash_algorithm, encryption_algorithm) {
        (HashAlgorithm::Argon2Id, Encryption::Aes) => {
            hash::argon2_with_salt(password, Aes256::key_size(), salt)
        }
        (HashAlgorithm::Argon2Id, Encryption::Camellia) => {
            hash::argon2_with_salt(password, Camellia256::key_size(), salt)
        }
        (HashAlgorithm::Pbkdf2Sha256, _) => hash::pbkdf2_with_salt(password, salt),
        (_, _) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                String::from("Invalid combination"),
            ))
        }
    };
    let hash = password_hash.hash.take().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            String::from("Failed to get hash"),
        )
    })?;
    Ok((hash, password_hash.to_string()))
}

fn encryption_writer<W: Write>(
    writer: W,
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
            CipherWriter::CtrAes(aes_ctr_cipher_writer(writer, key, iv)?)
        }
        (Encryption::Camellia, CipherMode::CBC) => {
            CipherWriter::CbcCamellia(EncryptCbcCamellia256Writer::new_with_iv(writer, key, iv)?)
        }
        (Encryption::Camellia, CipherMode::CTR) => {
            CipherWriter::CtrCamellia(camellia_ctr_cipher_writer(writer, key, iv)?)
        }
    })
}

fn compression_writer<'w, W: Write + 'w>(
    writer: W,
    algorithm: Compression,
    level: CompressionLevel,
) -> io::Result<CompressionWriter<'w, W>> {
    Ok(match algorithm {
        Compression::No => CompressionWriter::No(writer),
        Compression::Deflate => {
            CompressionWriter::Deflate(DeflateEncoder::new(writer, level.into()))
        }
        Compression::ZStandard => CompressionWriter::Zstd(ZstdEncoder::new(writer, level.into())?),
        Compression::XZ => CompressionWriter::Xz(XzEncoder::new(writer, level.into())),
    })
}

fn writer_and_hash<'a, W: Write + 'a>(
    writer: W,
    options: &'a WriteOption,
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

fn aes_ctr_cipher_writer<W: Write>(
    mut writer: W,
    key: &[u8],
    iv: &[u8],
) -> io::Result<Ctr128BEWriter<W, Aes256>> {
    writer.write_all(iv)?;
    Ctr128BEWriter::new(writer, key, iv)
}

fn camellia_ctr_cipher_writer<W: Write>(
    mut writer: W,
    key: &[u8],
    iv: &[u8],
) -> io::Result<Ctr128BEWriter<W, Camellia256>> {
    writer.write_all(iv)?;
    Ctr128BEWriter::new(writer, key, iv)
}
