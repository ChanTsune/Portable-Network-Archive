use crate::cipher::CipherWriter;
use crate::compress::CompressionWriter;
use crate::io::TryIntoInner;
use crate::{
    archive::{
        CipherMode, Compression, CompressionLevel, Encryption, HashAlgorithm, ItemName, Options,
        PNA_HEADER,
    },
    chunk::{self, ChunkWriter},
    cipher::{Ctr128BEWriter, EncryptCbcAes256Writer, EncryptCbcCamellia256Writer},
    create_chunk_data_ahed, create_chunk_data_fhed, hash, random,
};
use aes::Aes256;
use camellia::Camellia256;
use cipher::{BlockSizeUser, KeySizeUser};
use flate2::write::DeflateEncoder;
use password_hash::{Output, SaltString};
use std::io::{self, Write};
use xz2::write::XzEncoder;
use zstd::stream::write::Encoder as ZstdEncoder;

#[derive(Default)]
pub struct Encoder;

impl Encoder {
    pub fn new() -> Self {
        Self
    }

    pub fn write_header<W: Write>(&self, write: W) -> io::Result<ArchiveWriter<W>> {
        ArchiveWriter::write_header(write)
    }
}

pub struct ArchiveWriter<W: Write> {
    w: W,
    // temporary use fields
    options: Options,
    buf: Vec<u8>,
    file_closed: bool,
    // end temporary
    finalized: bool,
}

impl<W: Write> ArchiveWriter<W> {
    fn write_header(mut write: W) -> io::Result<Self> {
        write.write_all(PNA_HEADER)?;
        let mut chunk_writer = ChunkWriter::from(write);
        chunk_writer.write_chunk(chunk::AHED, &create_chunk_data_ahed(0, 0, 0))?;
        Ok(Self {
            w: chunk_writer.into_inner(),
            options: Options::default(),
            buf: Vec::new(),
            file_closed: true,
            finalized: false,
        })
    }

    pub fn start_file(&mut self, name: ItemName) -> io::Result<()> {
        self.start_file_with_options(name, Options::default())
    }

    pub fn start_file_with_options(&mut self, name: ItemName, options: Options) -> io::Result<()> {
        self.end_file()?;
        self.file_closed = false;
        self.options = options;

        let mut chunk_writer = ChunkWriter::from(&mut self.w);

        chunk_writer.write_chunk(
            chunk::FHED,
            &create_chunk_data_fhed(
                0,
                0,
                self.options.compression as u8,
                self.options.encryption as u8,
                self.options.cipher_mode as u8,
                0,
                name.as_ref(),
            ),
        )?;
        Ok(())
    }

    pub fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        self.buf.extend(data);
        Ok(())
    }

    pub fn end_file(&mut self) -> io::Result<()> {
        if self.file_closed {
            return Ok(());
        }

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

        // Write end of file
        chunk_writer.write_chunk(chunk::FEND, &[])?;
        self.file_closed = true;
        self.buf.clear();
        Ok(())
    }

    pub fn finalize(&mut self) -> io::Result<()> {
        self.end_file()?;
        if !self.finalized {
            let mut chunk_writer = ChunkWriter::from(&mut self.w);
            chunk_writer.write_chunk(chunk::AEND, &[])?;
            self.finalized = true;
        }
        Ok(())
    }
}

impl<W: Write> Drop for ArchiveWriter<W> {
    fn drop(&mut self) {
        self.finalize().expect("archive finalize failed.");
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
    options: &'a Options,
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

#[cfg(test)]
mod tests {
    use super::Encoder;

    #[test]
    fn encode() {
        let mut file = Vec::new();
        {
            let encoder = Encoder::new();
            let mut writer = encoder.write_header(&mut file).unwrap();
            writer.finalize().unwrap();
        }
        let expected = include_bytes!("../../../resources/empty_archive.pna");
        assert_eq!(file.as_slice(), expected.as_slice());
    }
}
