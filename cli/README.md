# Portable Network Archive
[![test](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml/badge.svg)](https://github.com/ChanTsune/Portable-Network-Archive/actions/workflows/test.yml)
[![Crates.io][crates-badge]][crates-url]
[![docs.rs](https://img.shields.io/docsrs/portable-network-archive)](https://docs.rs/portable-network-archive)

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

```powershell
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

Native PNA syntax never treats `-` as a standard-I/O sentinel: `--file -`, `--output -`, and
`--restore -` refer to a filesystem entry literally named `-`. This differs intentionally from
`pna compat bsdtar`. An explicit output is no-clobber by default; add `--overwrite` to replace it.
For rewrite commands, `--overwrite` without `--output` means in-place replacement of the `--file`
input and is rejected for stdin.

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

# License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](../LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license ([LICENSE-MIT](../LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
