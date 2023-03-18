use crate::{
    archive::{entry::EntryWriter, EntryName, WriteOption, WriteOptionBuilder, PNA_HEADER},
    chunk::{ChunkType, ChunkWriter},
    create_chunk_data_ahed, Entry,
};
use std::io::{self, Write};

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
    inner: Option<EntryWriter<Vec<u8>>>,
    // end temporary
    finalized: bool,
}

impl<W: Write> ArchiveWriter<W> {
    fn write_header(mut write: W) -> io::Result<Self> {
        write.write_all(PNA_HEADER)?;
        let mut chunk_writer = ChunkWriter::from(write);
        chunk_writer.write_chunk(ChunkType::AHED, &create_chunk_data_ahed(0, 0, 0))?;
        Ok(Self {
            w: chunk_writer.into_inner(),
            inner: None,
            finalized: false,
        })
    }

    pub fn start_file(&mut self, name: EntryName) -> io::Result<()> {
        self.start_file_with_options(name, WriteOptionBuilder::default().build())
    }

    pub fn start_file_with_options(
        &mut self,
        name: EntryName,
        options: WriteOption,
    ) -> io::Result<()> {
        self.end_file()?;
        self.inner = Some(EntryWriter::new_file_with(Vec::new(), name, options)?);
        Ok(())
    }

    pub fn write_all(&mut self, data: &[u8]) -> io::Result<()> {
        self.inner.as_mut().unwrap().write_all(data)?;
        Ok(())
    }

    pub fn end_file(&mut self) -> io::Result<()> {
        if let Some(item_writer) = self.inner.take() {
            let w = item_writer.finish()?;
            self.w.write_all(&w)?;
        }
        Ok(())
    }

    pub fn add_entry(&mut self, entry: impl Entry) -> io::Result<()> {
        self.w.write_all(&entry.into_bytes())
    }

    pub fn finalize(&mut self) -> io::Result<()> {
        self.end_file()?;
        if !self.finalized {
            let mut chunk_writer = ChunkWriter::from(&mut self.w);
            chunk_writer.write_chunk(ChunkType::AEND, &[])?;
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
        let expected = include_bytes!("../../../resources/test/empty.pna");
        assert_eq!(file.as_slice(), expected.as_slice());
    }
}
