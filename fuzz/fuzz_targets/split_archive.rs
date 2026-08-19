#![no_main]

use libfuzzer_sys::fuzz_target;
use libpna::{
    Archive, Compression, FileEntryBuilder, MIN_SPLIT_PART_BYTES, ReadOptions, WriteOptions,
};
use std::io::prelude::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Default)]
struct Part(Rc<RefCell<Vec<u8>>>);

impl Write for Part {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fuzz_target!(|input: (&[u8], usize)| {
    let (data, max_part_bytes) = input;
    // `max_part_bytes` comes straight from the fuzz input; cap it so parts stay
    // small enough to actually exercise the cutting/rollover paths.
    if !(MIN_SPLIT_PART_BYTES..=1 << 20).contains(&max_part_bytes) {
        return;
    }

    let write_option = WriteOptions::builder().compression(Compression::NO).build();
    let mut builder = FileEntryBuilder::new_with_options("fuzz".into(), write_option).unwrap();
    builder.write_all(data).unwrap();
    let entry = builder.build().unwrap();

    let parts: Rc<RefCell<Vec<Part>>> = Rc::new(RefCell::new(Vec::new()));
    let handle = parts.clone();
    let mut archive = Archive::write_split_header(max_part_bytes, move |_| {
        let part = Part::default();
        handle.borrow_mut().push(part.clone());
        Ok(part)
    })
    .unwrap();
    match archive.add_entry(entry) {
        Ok(_) => {}
        // The entry's chunks can never fit within `max_part_bytes`.
        Err(e) if e.kind() == std::io::ErrorKind::InvalidInput => return,
        Err(e) => panic!("{e}"),
    }
    let reported_parts = archive.finalize().unwrap().parts();

    // Collect the part bytes up front so each slice outlives the chained
    // `Archive` that borrows it across loop iterations.
    let part_bytes: Vec<Vec<u8>> = parts
        .borrow()
        .iter()
        .map(|p| std::mem::take(&mut *p.0.borrow_mut()))
        .collect();

    assert_eq!(reported_parts as usize, part_bytes.len());
    for part in &part_bytes {
        assert!(part.len() <= max_part_bytes, "{} bytes", part.len());
    }

    let mut archive = Archive::read_header(&part_bytes[0][..]).unwrap();
    let mut index = 1;
    let mut restored = Vec::with_capacity(data.len());
    loop {
        for entry in archive.entries().skip_solid() {
            let entry = entry.unwrap();
            let mut reader = entry.reader(ReadOptions::builder().build()).unwrap();
            reader.read_to_end(&mut restored).unwrap();
        }
        if !archive.has_next_archive() {
            break;
        }
        archive = archive.read_next_archive(&part_bytes[index][..]).unwrap();
        index += 1;
    }
    assert_eq!(data, &restored[..]);
});
