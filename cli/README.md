# Portable Network Archive
[![test](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml/badge.svg)](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml)
[![Crates.io][crates-badge]][crates-url]

[crates-badge]: https://img.shields.io/crates/v/portable-network-archive.svg
[crates-url]: https://crates.io/crates/portable-network-archive

PNA (Portable Network Archive) is a highly scalable archive format that can be compressed, encrypted, and split.
Also, its data structure is inspired by the PNG data structure.

## Installation

### Via Shell (Prebuilt Binary)

#### On Linux or macOS

```sh
curl --proto '=https' --tlsv1.2 -LsSf 'https://github.com/ChanTsune/Portable-Network-Archive/releases/latest/download/portable-network-archive-installer.sh' | sh
```

#### On Windows

```sh
powershell -ExecutionPolicy Bypass -c "irm https://github.com/ChanTsune/Portable-Network-Archive/releases/latest/download/portable-network-archive-installer.ps1 | iex"
```

### Via Cargo

```sh
cargo install portable-network-archive
```

### From Source (via Cargo)

```sh
cargo install --git https://github.com/ChanTsune/Portable-Network-Archive.git portable-network-archive
```

## Usage

Note on archive argument style
- The positional archive argument `<ARCHIVE>` is deprecated since version 0.28.0. Use `-f/--file <ARCHIVE>` instead. The positional form is still accepted for backward compatibility and will emit a warning. It may be removed in a future release.

### Creating an Archive

```sh
pna create -f <ARCHIVE> [FILES]...
```

### Extracting an Archive

```sh
pna extract -f <ARCHIVE>
```

### Listing Archived Entries

```sh
pna list -f <ARCHIVE>
```

Use the following command to get help.

```sh
pna --help
```

# License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](../LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](../LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
