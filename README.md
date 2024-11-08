# Portable Network Archive

<div align="center">
  <img src="./icon.svg" alt="PNA" width="100"/>
</div>

PNA (Portable Network Archive) is a highly scalable archive format that can be compressed, encrypted, and split.
Also, its data structure is inspired by the PNG data structure.

### Why PNA?

**Portable Network Archive (PNA): A Flexible, Secure, and Cross-Platform Archive Format**
- **Portability:** Works seamlessly across multiple platforms, combining the strengths of TAR and ZIP formats.
- **Compression Flexibility:** Advanced per-file and archive-wide compression options reduce the need for full archive decompression.
- **Encryption & Security:** Supports 256-bit AES and Camellia for robust protection of sensitive data.
- **Splittable Structure**: Based on PNG’s data unit structure, enabling the easy division of large archives into smaller parts.
- **Streamability:** Supports serial read and write operations, making it suitable streaming processing, similar to a TAR format.
- **Extensibility**: Designed to accommodate future extensions and private add-ons, ensuring compatibility with the basic PNA format while allowing for flexible customization.
- **Error Resilience:** File integrity checks and error detection ensure data is secure during transmission.

Additionally, the PNA specification includes a rationale appendix to help developers understand key design choices, making implementation more straightforward.

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

## Supported Operating Systems

PNA (Portable Network Archive) CLI is cross-platform and compatible with the following operating systems:

- **Windows**
- **Linux**
- **macOS**
- **FreeBSD**

This compatibility ensures that users can utilize PNA across different environments without any platform-specific limitations.

Further supported operating systems will be added in the future.

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
