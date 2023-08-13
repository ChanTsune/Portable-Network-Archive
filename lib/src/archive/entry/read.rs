use crate::cipher::Ctr128BEReader;
use aes::Aes256;
use camellia::Camellia256;
use crypto_common::BlockSizeUser;
use std::io;
use std::io::Read;
use std::sync::Mutex;

// NOTE: zstd crate not support Sync + Send trait
pub(crate) struct MutexRead<R: Read>(Mutex<R>);

impl<R: Read> MutexRead<R> {
    pub(super) fn new(reader: R) -> Self {
        Self(Mutex::new(reader))
    }
}

impl<R: Read> Read for MutexRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let reader = self.0.get_mut().unwrap();
        reader.read(buf)
    }
}

pub(super) fn aes_ctr_cipher_reader<R: Read>(
    mut reader: R,
    key: &[u8],
) -> io::Result<Ctr128BEReader<R, Aes256>> {
    let mut iv = vec![0u8; Aes256::block_size()];
    reader.read_exact(&mut iv)?;
    Ctr128BEReader::new(reader, key, &iv)
}

pub(super) fn camellia_ctr_cipher_reader<R: Read>(
    mut reader: R,
    key: &[u8],
) -> io::Result<Ctr128BEReader<R, Camellia256>> {
    let mut iv = vec![0u8; Camellia256::block_size()];
    reader.read_exact(&mut iv)?;
    Ctr128BEReader::new(reader, key, &iv)
}
