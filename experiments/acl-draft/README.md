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
| Windows NT 0x0003 | Yes (basic descriptor structure) | Yes | Windows native adapter and distinct-file DACL roundtrip test pass on prior HEAD; full descriptor/SACL semantic equivalence is not yet established |
| Darwin 0x0004 | Yes (basic structure) | Yes | macOS native adapter and legacy compatibility tests pass on prior HEAD; complete native semantic equivalence is not yet established |
| Extended NFSv4.1 0x0005 | Yes (XDR structural) | Yes | Not implemented |
| Unknown format ID | Preserved opaque | Yes | Not applied |

**Validation is not authorization equivalence.** The proof of binary roundtrip does not establish that all supported OS ACLs can be extracted losslessly, installed successfully, or have identical effective access checks.

## CI evidence and known gaps (2026-11-22)

- Latest inspected branch HEAD: `fc0df239a19ac94666a58118af75b459f7555a14`. [Format run 44951055913](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44951055913) passed; [Test run 44951056016](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44951056016) was queued and [WASM run 44951055915](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44951055915) was in progress when checked. These are not yet evidence of a green HEAD.
- Prior `fdd11c3e`: [Test run 44944968437](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44944968437) passed all 21 jobs (Linux, macOS and Windows toolchains, including Windows x64/ARM64), [WASM run 44944968429](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44944968429) passed, but [Format run 44944968522](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44944968522) failed. HEAD `fc0df239` fixes only that formatting difference.
- Windows distinct-file DACL capture → PNA → restoration → recapture passes in the prior test matrix. The test does not prove the full DACL/SACL/owner/group-free descriptor validity matrix, effective authorization equivalence, or SACL category coverage. The native setter fallback also requires explicit verification of which API succeeded.
- FreeBSD [run 44908816452](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44908816452) passed at workflow level, but the NFSv4 native test explicitly returns success on unsupported filesystems (`SKIP NFSv4 native filesystem test`). The [FreeBSD workflow](../../.github/workflows/acl-draft-freebsd.yml) currently uses the VM default filesystem and is path-filtered, so it is not triggered for every HEAD. A dedicated ACL-capable UFS/ZFS filesystem and a mandatory non-skipping native roundtrip are still required.
- The current FreeBSD POSIX numeric-principal unit test constructs an ACL in memory; it does **not** apply a POSIX/default ACL to a real filesystem or verify a separate-object archive restoration.
- Do not count a skipped test, a structural-only comparison, or a previously green commit as proof of full native roundtrip.

## Important unresolved draft issues

1. **Windows:** confirm owner/group-free self-relative descriptor validity, exact distinction of absent/NULL/empty ACL, DACL-only/SACL-only coverage controls, full-SACL acquisition semantics (including integrity labels), and inherited control bits with native roundtrips.
2. **Darwin:** confirm native external encoding alignment/bit masks, UUID principal type semantics, `NOACL` sentinel with `acl_flags`, all types and raw filesystem-private flags; native owner UUID chunks are a separate proposal.
3. **NFSv4/FreeBSD:** verify UUID/name/UID mapping and ACE flag set, especially inherited ACE, mode bits, dacl/sacl applicability and possible loss of unknown principal data; FreeBSD native bridges exist but NFSv4 filesystem validation is still missing.
4. **POSIX:** validate default ACL behavior on directories, numeric-ID and optional-name facets, symlinks/hardlinks, ACL vs `fMOd` ordering and consistency, and case where an ACL is absent but `fMOd` remains.
5. **Common/editor:** verify duplicate aCLS error handling without losing file contents, unsafe-to-copy rules when critical chunks are edited, unknown public/private IDs, preservation of after-FDAT placement, and encrypted/solid entries.
6. **CLI:** `pna acl get/set` still uses experimental legacy `faCl`/`faCe` and does not edit new `aCLS`. Model conversion is deliberately avoided. Non-Linux native adapters exist but remain experimental and unverified.

Do not merge into `main` before cross-platform tests and the draft's reserved-bit/validity rules are finalized.
