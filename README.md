# Portable Network Archive (PNA)

[![Crates.io](https://img.shields.io/crates/v/portable-network-archive.svg)](https://crates.io/crates/portable-network-archive)
[![Downloads](https://img.shields.io/crates/d/portable-network-archive.svg)](https://crates.io/crates/portable-network-archive)
[![Docs.rs](https://docs.rs/portable-network-archive/badge.svg)](https://docs.rs/portable-network-archive)
[![Test](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml/badge.svg)](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml)
![License](https://img.shields.io/crates/l/portable-network-archive.svg)

<div align="center">
  <img src="./icon.svg" alt="PNA" width="100"/>
</div>

**Portable Network Archive (PNA)** is a flexible, secure, and cross-platform archive format inspired by the PNG data structure. It combines the simplicity of ZIP with the robustness of TAR, providing efficient compression, strong encryption, and seamless splitting and streaming capabilities.

## Why PNA?

- **Metadata-free:** Unlike many archive formats, PNA requires nothing but the entry name and the entry body. Timestamps, permissions, owner IDs and every other field are optional, so you get the smallest possible archives, no accidental leaks of environment details (mtime, uid/gid, tool versions), and byte-for-byte reproducible output that makes reproducible builds easy.
- **Portability:** Works seamlessly across multiple platforms, combining the strengths of TAR and ZIP formats. The CLI supports Windows, Linux, macOS, and FreeBSD (support for additional platforms planned).
- **Compression Flexibility:** Supports zlib, zstd, and xz. Advanced per-file and archive-wide compression options reduce the need for full archive decompression. Solid Mode compresses and encrypts the entire archive as a single block.
- **Encryption & Security:** Supports 256-bit AES and 256-bit Camellia for robust protection of sensitive data.
- **Splittable Structure:** Based on PNG's data unit structure, enabling the easy division of large archives into smaller parts.
- **Streamability:** Supports serial read and write operations, making it suitable for streaming processing, similar to a TAR format.
- **Extensibility:** Designed to accommodate future extensions and private add-ons, ensuring compatibility with the basic PNA format while allowing for flexible customization.
- **Error Resilience:** File integrity checks and error detection ensure data is secure during transmission.
- **Attribute Preservation:** When you do want them, file permissions, timestamps, extended attributes, and Access Control Lists (ACLs, experimental) are maintained and restored.

Additionally, the PNA specification includes a rationale appendix to help developers understand key design choices, making implementation more straightforward.

## Installation

### Via Shell (Prebuilt Binary)

#### On Linux or macOS

```sh
curl --proto '=https' --tlsv1.2 -LsSf 'https://github.com/ChanTsune/Portable-Network-Archive/releases/latest/download/portable-network-archive-installer.sh' | sh
```

#### On Windows

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/ChanTsune/Portable-Network-Archive/releases/latest/download/portable-network-archive-installer.ps1 | iex"
```

### Via Cargo

**From crates.io:**

```sh
cargo install portable-network-archive
```

**From source:**

```sh
cargo install --git https://github.com/ChanTsune/Portable-Network-Archive.git portable-network-archive
```

## Usage

### PNA-native style

```sh
pna create -f archive.pna file1.txt file2.txt
pna extract -f archive.pna
pna list -f archive.pna
```

#### Native standard I/O and pipelines

For native commands that read an archive, omitting `--file` reads the PNA datastream from
standard input. Commands that produce one archive write it to standard output when their output
path is omitted.

```sh
# Create, inspect, and extract through a pipeline.
pna create file1.txt file2.txt > archive.pna
pna list < archive.pna
pna extract --out-dir restored < archive.pna

# Rewrites are non-mutating by default.
pna delete obsolete.txt < archive.pna > cleaned.pna
pna delete --file archive.pna obsolete.txt > cleaned.pna

# Mutation is explicit.
pna delete --file archive.pna --overwrite obsolete.txt

# Concat takes repeated inputs; omit --output for stdout.
pna concat --file part-a.pna --file part-b.pna > combined.pna
```

The former concat convention that treated the first `--file` as the output is removed; use
`--output combined.pna` when the result should be written to a file.

Native `--file -` and `--output -` refer to a filesystem entry literally named `-`, unlike
`pna compat bsdtar`. Use `--restore-from-stdin` to restore metadata from stdin; the deprecated
`xattr set --restore -` spelling also reads stdin during its compatibility period. An explicit output is no-clobber by default; add `--overwrite` to replace it.
For rewrite commands, `--overwrite` without `--output` means in-place replacement of the `--file`
input and is rejected for stdin.
For native rewrite commands, `--no-overwrite` remains available with `--output` for
compatibility, but is deprecated and emits a warning. Omit it to keep the default
no-clobber behavior; use `--overwrite` only when replacement is intended.

Filesystem replacement is staged and committed only after success. Standard output cannot be
transactional, so a failing command or a consumer that closes early may leave the consumer with a
partial stream; use a temporary file and rename it when atomic publication is required. A command
also rejects combinations that assign stdin to both the archive and an auxiliary option such as
`--files-from-stdin`.

Multipart discovery is filesystem-only. A `--file` source can discover its numbered parts, while
stdin contains only the sequential bytes supplied by the caller. Ordinary rewrites consolidate a
multipart input when writing to stdout or `--output`; in-place multipart rewrites are rejected.
`create --split` and `split` always produce filesystem files, and splitting stdin requires an
explicit `split --output BASE_PATH` for part names.

### tar-like style

If you prefer tar-like syntax, a bsdtar-compatible interface is available:

```sh
pna compat bsdtar -cf archive.pna file1.txt file2.txt
pna compat bsdtar -xf archive.pna
pna compat bsdtar -tf archive.pna
```

Both styles produce PNA-format archives. Note that `compat bsdtar` preserves permissions, ownership, and timestamps by default (matching bsdtar behavior), while PNA-native commands require explicit flags to preserve them.

For more commands and options:
```sh
pna --help
```

See also the [CLI Reference](./docs/cli-reference.md) for detailed command documentation.

## Specification

Detailed information is available in the [Specification](https://portable-network-archive.github.io/Portable-Network-Archive-Specification/) document.

# License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](./LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](./LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
