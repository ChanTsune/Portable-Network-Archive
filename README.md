# Portable Network Archive

<div align="center">
  <img src="./icon.svg" alt="PNA" width="100"/>
</div>

PNA (Portable Network Archive) is a highly scalable archive format that can be compressed, encrypted, and split.
Also, its data structure is inspired by the PNG data structure.

## Features

- **File Compression and Decompression**
  - [x] Supports zlib, zstd, and xz.

- **File Encryption and Decryption**
  - [x] Supports 256-bit AES and 256-bit Camellia.

- **Solid Mode**
  - [x] Compresses and encrypts the entire archive as a single block.

- **File Attribute Preservation (Maintains and restores)**
  - [x] File permissions.
  - [x] File timestamps.
  - [x] Extended attributes.
  - [x] Access Control Lists (ACLs) (experimental).

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

### Extracting an Archive

```sh
pna extract <ARCHIVE>
```

### Listing Archived Entries

```sh
pna list <ARCHIVE>
```

Use the following command to get help.

```sh
pna --help
```

## Specification

For more detailed information, please refer to the [Specification](https://portable-network-archive.github.io/Portable-Network-Archive-Specification/) document.

# License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](./LICENSE-APACHE) or
http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](./LICENSE-MIT) or
http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
