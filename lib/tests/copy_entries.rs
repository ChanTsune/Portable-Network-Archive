use libpna::Archive;
use std::io::{self, Read, Write};

/// Copy the entries to another archive
fn copy_entries<R: Read, W: Write>(r: R, w: W) -> io::Result<()> {
    let mut writer = Archive::write_header(w)?;
    let mut reader = Archive::read_header(r)?;
    for entry in reader.entries() {
        writer.add_entry(entry?)?;
    }
    writer.finalize()?;
    Ok(())
}

#[test]
fn copy() {
    let src = include_bytes!("../../resources/test/deflate.pna");
    let mut dist = Vec::new();
    copy_entries(src.as_slice(), &mut dist).unwrap();
    assert_eq!(src.as_slice(), dist.as_slice());
}
