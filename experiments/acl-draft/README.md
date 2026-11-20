# ACL draft format experiment

This branch is an experiment against the unmerged [aCLS draft specification](https://github.com/Portable-Network-Archive/Portable-Network-Archive-Specification/blob/draft/acl-specification/drafts/acl.md). Do not treat the format IDs or code as stable.

## Scope and how to exercise

- `libpna::acl::AclChunk` preserves `format_id:u16` and opaque model payloads, validates the five proposed public formats, and exposes `NormalEntry::acl_chunk()`.
- `PosixAcl` provides a lossless **binary** roundtrip for POSIX access/default ACLs, including named numeric qualifiers, optional UTF-8 names and MASK.
- Linux builds of the CLI with the `acl` feature use POSIX.1e `aCLS` on `pna create --preserve-acls` and restore it with `pna extract --preserve-acls`. Native numeric UID/GID is used and optional principal names are not captured. The old private ACL chunks are still recognized on extraction.
- Other operating systems retain the old private-chunk **native capture** path in this experiment; they can store/read any draft-format `aCLS` using libpna's raw extra chunks, but do **not** yet capture/restore the new formats natively.
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
| POSIX.1e 0x0001 | Yes | Yes | Linux only (requires CI/runtime validation) |
| Extended NFSv4 0x0002 | Yes (XDR structural) | Yes | Not implemented |
| Windows NT 0x0003 | Yes (basic descriptor structure) | Yes | Not implemented |
| Darwin 0x0004 | Yes (basic structure) | Yes | Not implemented |
| Extended NFSv4.1 0x0005 | Yes (XDR structural) | Yes | Not implemented |
| Unknown format ID | Preserved opaque | Yes | Not applied |

**Validation is not authorization equivalence.** The proof of binary roundtrip does not establish that all supported OS ACLs can be extracted losslessly, installed successfully, or have identical effective access checks.

## Important unresolved draft issues

1. **Windows:** confirm owner/group-free self-relative descriptor validity, exact distinction of absent/NULL/empty ACL, DACL-only/SACL-only coverage controls, full-SACL acquisition semantics (including integrity labels), and inherited control bits with native roundtrips.
2. **Darwin:** confirm native external encoding alignment/bit masks, UUID principal type semantics, `NOACL` sentinel with `acl_flags`, all types and raw filesystem-private flags; native owner UUID chunks are a separate proposal.
3. **NFSv4/FreeBSD:** verify UUID/name/UID mapping and ACE flag set, especially inherited ACE, mode bits, dacl/sacl applicability and possible loss of unknown principal data; no native FreeBSD bridge exists here.
4. **POSIX:** validate default ACL behavior on directories, numeric-ID and optional-name facets, symlinks/hardlinks, ACL vs `fMOd` ordering and consistency, and case where an ACL is absent but `fMOd` remains.
5. **Common/editor:** verify duplicate aCLS error handling without losing file contents, unsafe-to-copy rules when critical chunks are edited, unknown public/private IDs, preservation of after-FDAT placement, and encrypted/solid entries.
6. **CLI:** `pna acl get/set` still uses experimental legacy `faCl`/`faCe` and does not edit new `aCLS`. Model conversion is deliberately avoided. Non-Linux native adapters are intentionally incomplete.

Do not merge into `main` before cross-platform tests and the draft's reserved-bit/validity rules are finalized.
