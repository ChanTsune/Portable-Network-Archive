# Portable Network Archive
[![test](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml/badge.svg)](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml)
[![Crates.io][crates-badge]][crates-url]

[crates-badge]: https://img.shields.io/crates/v/portable-network-archive.svg
[crates-url]: https://crates.io/crates/portable-network-archive

PNA (Portable Network Archive) is a highly scalable archive format that can be compressed, encrypted, and split.
Also, its data structure is inspired by the PNG data structure.

## Installation

### Via Cargo

```sh
cargo install portable-network-archive
```

### From Source (via Cargo)

```sh
cargo install --git https://github.com/ChanTsune/Portable-Network-Archive.git portable-network-archive
```

## Usage

### Creating an Archive

```sh
pna create <ARCHIVE> [FILES]...
```

**Options for `create`:**
- `--store-windows-attributes` (Windows-only): When creating an archive on Windows, this flag stores specific file attributes (such as ReadOnly, Hidden, System) as an extended attribute named `windows.file_attributes`. This allows these attributes to be preserved and restored on other Windows systems.

### Extracting an Archive

```sh
pna extract <ARCHIVE>
```

**Options for `extract`:**
- `--restore-windows-attributes` (Windows-only): When extracting an archive on Windows, this flag restores the Windows-specific file attributes (e.g., ReadOnly, Hidden) to the extracted files, provided they were stored during creation (e.g., with `--store-windows-attributes`).

### Windows File Attributes (xattr)

When using the `--store-windows-attributes` flag on Windows during archive creation, the file attributes are stored as an extended attribute (xattr) with the name `windows.file_attributes`. The value of this xattr is a hexadecimal string representing the Windows file attribute DWORD (e.g., `0x21` for ReadOnly and Hidden). This xattr can be inspected using `pna xattr get <ARCHIVE> <FILE> --name windows.file_attributes --encoding hex`.

### Listing Archived Entries

```sh
pna list <ARCHIVE>
```

Use the following command to get help.

```sh
pna --help
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
