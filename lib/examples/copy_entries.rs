use libpna::Archive;
use std::io::{self, Read, Write};

/// Simply copy the entries to another archive
fn copy_entries<R: Read, W: Write>(r: R, w: W) -> io::Result<()> {
    let mut writer = Archive::write_header(w)?;
    let mut reader = Archive::read_header(r)?;
    for entry in reader.entries() {
        writer.add_entry(entry?)?;
    }
    writer.finalize()?;
    Ok(())
}

fn main() -> io::Result<()> {
    let mut dist = vec![];
    copy_entries(
        include_bytes!("../../resources/test/deflate.pna").as_slice(),
        &mut dist,
    )
}
