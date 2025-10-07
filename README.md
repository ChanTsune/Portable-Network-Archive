# Portable Network Archive (PNA)

[![Crates.io](https://img.shields.io/crates/v/portable-network-archive.svg)](https://crates.io/crates/portable-network-archive)
[![Downloads](https://img.shields.io/crates/d/portable-network-archive.svg)](https://crates.io/crates/portable-network-archive)
[![Docs.rs](https://docs.rs/portable-network-archive/badge.svg)](https://docs.rs/portable-network-archive)
[![Test](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml/badge.svg)](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml)
![License](https://img.shields.io/crates/l/portable-network-archive.svg)

<div align="center">
  <img src="./icon.svg" alt="PNA" width="100"/>
</div>

**Portable Network Archive (PNA)** is a modern, flexible, and secure archive format designed for cross-platform use. Inspired by the chunk-based structure of PNG, PNA combines the simplicity of ZIP with the robustness of TAR, offering efficient compression, strong encryption, and advanced features like seamless splitting and streaming.

## Why PNA?

PNA was created to address the limitations of existing archive formats, providing a single, powerful tool that excels in a variety of use cases:

- **Portability:** PNA archives work seamlessly across different operating systems, preserving file attributes like permissions and timestamps.
- **Flexibility:** Choose from multiple compression algorithms (zlib, zstd, xz) and encryption ciphers (AES-256, Camellia-256) on a per-file or archive-wide basis.
- **Security:** Robust 256-bit encryption protects your sensitive data, with support for modern password hashing algorithms like Argon2.
- **Efficiency:** The chunk-based structure allows for streaming operations and easy splitting of large archives, without requiring the entire file to be held in memory.
- **Extensibility:** The format is designed to be extended with custom chunks, ensuring future compatibility while allowing for specialized features.
- **Resilience:** File integrity is ensured through CRC checksums, protecting your data against corruption during transfer or storage.

## Features

- **Advanced Compression:** Supports zlib, zstd, and xz, allowing you to balance speed and compression ratio.
- **Strong Encryption:** Protect your data with AES-256 or Camellia-256 encryption, using either CBC or CTR mode.
- **Solid Mode:** Achieve higher compression ratios by compressing all files in an archive as a single block.
- **Attribute Preservation:** Faithfully preserves file permissions, timestamps, extended attributes, and experimental support for Access Control Lists (ACLs).
- **Cross-Platform CLI:** A single, easy-to-use command-line interface for all supported platforms, including Windows, Linux, macOS, and FreeBSD.

## Installation

### Pre-built Binaries (Recommended)

The easiest way to get started with PNA is to download a pre-built binary for your operating system.

#### Linux or macOS

```sh
curl --proto '=https' --tlsv1.2 -LsSf 'https://github.com/ChanTsune/Portable-Network-Archive/releases/latest/download/portable-network-archive-installer.sh' | sh
```

#### Windows

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/ChanTsune/Portable-Network-Archive/releases/latest/download/portable-network-archive-installer.ps1 | iex"
```

### Via Cargo

If you have the Rust toolchain installed, you can install PNA using `cargo`:

```sh
cargo install portable-network-archive
```

### Building from Source

To build PNA from source, you'll need the [Rust toolchain](https://www.rust-lang.org/tools/install) (1.65 or later).

1.  Clone the repository:
    ```sh
    git clone https://github.com/ChanTsune/Portable-Network-Archive.git
    cd Portable-Network-Archive
    ```

2.  Build the project:
    ```sh
    cargo build --release
    ```

3.  The executable will be located at `target/release/pna`.

## Usage

The `pna` command-line tool provides a range of commands for working with archives. Here are some common examples:

### Creating an Archive

To create a new archive from a set of files:

```sh
pna create -f my_archive.pna file1.txt path/to/directory/
```

### Creating an Encrypted Archive

You can encrypt an archive with a password using the `--aes` or `--camellia` flags. If you don't provide a password on the command line, you will be prompted for one.

```sh
# Create an AES-encrypted archive
pna create -f secure.pna --aes --password "my_secret_password" private_document.txt

# Create a Camellia-encrypted archive and be prompted for a password
pna create -f secure.pna --camellia --password
```

### Using Different Compression Algorithms

PNA defaults to Zstandard for a good balance of speed and compression. You can specify a different algorithm and compression level:

```sh
# Use Deflate (zlib) compression at level 9 (maximum)
pna create -f archive.pna --deflate 9 file.log

# Store files without compression
pna create -f archive.pna --store image.jpg
```

### Listing Archive Contents

To see the files inside an archive:

```sh
pna list -f my_archive.pna
```

### Extracting an Archive

To extract all files from an archive:

```sh
pna extract -f my_archive.pna
```

To extract specific files, simply list them after the command:

```sh
pna extract -f my_archive.pna path/to/file1.txt path/to/file2.txt
```

### Appending to an Archive

You can add more files to an existing archive with the `append` command:

```sh
pna append -f my_archive.pna new_file.txt
```

For a full list of commands and options, run:

```sh
pna --help
```

## Contributing

Contributions are welcome! If you'd like to help improve PNA, please follow these steps:

1.  **Fork the repository** and clone it to your local machine.
2.  **Create a new branch** for your feature or bug fix: `git checkout -b my-new-feature`.
3.  **Make your changes.** Please ensure your code follows the project's style and conventions.
4.  **Run the tests** to make sure everything is working correctly: `cargo test --workspace`.
5.  **Check for linting issues** with `clippy`: `cargo clippy --workspace --all-targets -- -D warnings`.
6.  **Commit your changes** with a clear and descriptive commit message.
7.  **Push your branch** to your fork and open a pull request.

Please also see the [contribution guidelines](https://github.com/ChanTsune/Portable-Network-Archive/blob/main/CONTRIBUTING.md) for more details.

## Specification

For a detailed breakdown of the PNA archive format, please see the official [Specification](https://portable-network-archive.github.io/Portable-Network-Archive-Specification/) document.

## License

This project is licensed under either of

*   Apache License, Version 2.0, ([LICENSE-APACHE](./LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
*   MIT license ([LICENSE-MIT](./LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.