# pna
[![test](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml/badge.svg)](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml)
[![Crates.io][crates-badge]][crates-url]

[crates-badge]: https://img.shields.io/crates/v/pna.svg
[crates-url]: https://crates.io/crates/pna

A pna archive reading/writing library for Rust.

```toml
# Cargo.toml
[dependencies]
pna = "0.25"
```

## Reading an archive

```rust
use pna::{Archive, ReadOptions};
use std::fs::File;
use std::io::{self, copy, prelude::*};

fn main() -> io::Result<()> {
    let file = File::open("foo.pna")?;
    let mut archive = Archive::read_header(file)?;
    for entry in archive.entries_skip_solid() {
        let entry = entry?;
        let mut file = File::create(entry.header().path().as_path())?;
        let mut reader = entry.reader(ReadOptions::builder().build())?;
        copy(&mut reader, &mut file)?;
    }
    Ok(())
}
```

## Writing an archive

```rust
use pna::{Archive, EntryBuilder, WriteOptions};
use std::fs::File;
use std::io::{self, prelude::*};

fn main() -> io::Result<()> {
    let file = File::create("foo.pna")?;
    let mut archive = Archive::write_header(file)?;
    let mut entry_builder = EntryBuilder::new_file(
        "bar.txt".into(),
        WriteOptions::builder().build(),
    )?;
    entry_builder.write(b"content")?;
    let entry = entry_builder.build()?;
    archive.add_entry(entry)?;
    archive.finalize()?;
    Ok(())
}
```

# CLI
Command line user interface are [available](https://crates.io/crates/portable-network-archive), and you can install via cargo or build from source.

### Via Cargo

```sh
cargo install portable-network-archive
```

### From Source (via Cargo)

```sh
cargo install --git https://github.com/ChanTsune/Portable-Network-Archive.git portable-network-archive
```

# License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](../LICENSE-APACHE) or
http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](../LICENSE-MIT) or
http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
