# Portable Network Archive

<div align="center">
  <img src="./icon.svg" alt="PNA" width="100"/>
</div>

PNA (Portable Network Archive) is a highly scalable archive format that can be compressed, encrypted, and split.
Also, its data structure is based on the PNG data structure.

## Installation

### Via Cargo

```sh
cargo install portable-network-archive
```

### From Source (via Cargo)

```sh
git clone https://github.com/ChanTsune/Portable-Network-Archive.git
```

```sh
cargo install --path cli
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

For more detailed information, please refer to the [Specification](./Specification.md) document.
