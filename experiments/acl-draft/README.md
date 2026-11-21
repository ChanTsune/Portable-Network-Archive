# ACL draft format experiment

This branch is an experiment against the unmerged [aCLS draft specification](https://github.com/Portable-Network-Archive/Portable-Network-Archive-Specification/blob/draft/acl-specification/drafts/acl.md). Do not treat the format IDs or code as stable.

## Scope and how to exercise

- `libpna::acl::AclChunk` preserves `format_id:u16` and opaque model payloads, validates the five proposed public formats, and exposes `NormalEntry::acl_chunk()`.
- `PosixAcl` provides a lossless **binary** roundtrip for POSIX access/default ACLs, including named numeric qualifiers, optional UTF-8 names and MASK.
- Linux builds of the CLI with the `acl` feature use POSIX.1e `aCLS` on `pna create --preserve-acls` and restore it with `pna extract --preserve-acls`. Native numeric UID/GID is used and optional principal names are not captured. The old private ACL chunks are still recognized on extraction.
- Windows and macOS have experimental native `aCLS` capture/restore adapters and also emit legacy `faCl`/`faCe` chunks for compatibility. FreeBSD has separate NFSv4 and POSIX.1e adapters. Presence of an adapter does **not** establish a verified, lossless native roundtrip.
- The standalone, independently encoded `resources/test/acl_draft_probe.pna` contains six entries, with format IDs 1–5 and private ID 0x8001, including `aCLS` after `FDAT`. Tests in `lib/src/acl.rs` read this fixture, check each model's structure, and decode file contents. The fixture was also verified by an independent PNA chunk reader with CRC checks.

Local tests (with Rust installed):

```sh
cargo fmt --check
cargo test -p libpna acl::tests
cargo test -p portable-network-archive --features acl --lib acl_draft::tests
cargo test -p portable-network-archive --features acl --test cli
```

To exercise native Linux file permissions (requires a filesystem with POSIX ACL support):

```sh
cargo run -p portable-network-archive --features acl -- create --preserve-acls -f example.pna file.txt
cargo run -p portable-network-archive --features acl -- extract --preserve-acls -f example.pna
```

The named-user/group test is in `cli/src/acl_draft.rs`; it writes an actual PNA archive and applies its ACL to a second file.

## Validation matrix

| Format | Read/validate envelope | Binary PNA fixture | Native capture + restoration |
|---|---|---|---|
| POSIX.1e 0x0001 | Yes | Yes | Linux native adapter and FreeBSD POSIX adapter; native equivalence not fully proven |
| Extended NFSv4 0x0002 | Yes (XDR structural) | Yes | FreeBSD native adapter exists; CI filesystem lacks NFSv4 support, so native test currently skips |
| Windows NT 0x0003 | Yes (basic descriptor structure) | Yes | Windows native adapter exists; prior CI failed with OS error 998, and semantic equivalence is not yet established |
| Darwin 0x0004 | Yes (basic structure) | Yes | macOS native adapter exists; prior legacy compatibility tests failed; later fix awaits CI verification |
| Extended NFSv4.1 0x0005 | Yes (XDR structural) | Yes | Not implemented |
| Unknown format ID | Preserved opaque | Yes | Not applied |

**Validation is not authorization equivalence.** The proof of binary roundtrip does not establish that all supported OS ACLs can be extracted losslessly, installed successfully, or have identical effective access checks.

## CI evidence and known gaps (2026-11-21)

- Latest observed HEAD before this documentation update: `5b130f8ec1735d0d6d03c100e60b88b9c419dbb3`. Its format, test, FreeBSD and WASM workflows were still queued at the time of inspection. Do not substitute older runs for HEAD validation.
- FreeBSD workflow [44873010267](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44873010267) succeeded at the job level, but printed `SKIP NFSv4 native filesystem test: filesystem does not support NFSv4 ACLs`. The test returns success on this skip; the job is **not** proof of NFSv4 native equivalence. A suitable NFSv4-enabled filesystem and a non-skippable assertion are required.
- Earlier Windows/macOS workflow [44872927040](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44872927040) failed. Later Windows ACL alignment and macOS legacy-compatibility changes must be independently retested on current HEAD.
- Windows currently uses an owner/group-free self-relative descriptor with `SetSecurityInfo` on restoration. Its test checks structural validity after recapture, not exact ACL semantic equality. The preferred design is a fully native, valid security descriptor; do not switch to a PNA-defined descriptor envelope without strong native evidence that the preferred design cannot work.
- Do not count a skipped test, a structural-only comparison, or a previously green commit as proof of full native roundtrip.

## Important unresolved draft issues

1. **Windows:** confirm owner/group-free self-relative descriptor validity, exact distinction of absent/NULL/empty ACL, DACL-only/SACL-only coverage controls, full-SACL acquisition semantics (including integrity labels), and inherited control bits with native roundtrips.
2. **Darwin:** confirm native external encoding alignment/bit masks, UUID principal type semantics, `NOACL` sentinel with `acl_flags`, all types and raw filesystem-private flags; native owner UUID chunks are a separate proposal.
3. **NFSv4/FreeBSD:** verify UUID/name/UID mapping and ACE flag set, especially inherited ACE, mode bits, dacl/sacl applicability and possible loss of unknown principal data; FreeBSD native bridges exist but NFSv4 filesystem validation is still missing.
4. **POSIX:** validate default ACL behavior on directories, numeric-ID and optional-name facets, symlinks/hardlinks, ACL vs `fMOd` ordering and consistency, and case where an ACL is absent but `fMOd` remains.
5. **Common/editor:** verify duplicate aCLS error handling without losing file contents, unsafe-to-copy rules when critical chunks are edited, unknown public/private IDs, preservation of after-FDAT placement, and encrypted/solid entries.
6. **CLI:** `pna acl get/set` still uses experimental legacy `faCl`/`faCe` and does not edit new `aCLS`. Model conversion is deliberately avoided. Non-Linux native adapters exist but remain experimental and unverified.

Do not merge into `main` before cross-platform tests and the draft's reserved-bit/validity rules are finalized.
