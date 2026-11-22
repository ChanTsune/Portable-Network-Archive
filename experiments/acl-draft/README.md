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
| POSIX.1e 0x0001 | Yes | Yes | Linux native adapter and FreeBSD POSIX adapter; FreeBSD POSIX.1e directory access/default ACL native archive roundtrip passed on dedicated UFS; wider semantics remain unverified |
| Extended NFSv4 0x0002 | Yes (XDR structural) | Yes | FreeBSD native adapter and distinct-file archive roundtrip passed on a dedicated UFS filesystem with nfsv4acls; broader ACE semantics unverified |
| Windows NT 0x0003 | Yes (basic descriptor structure) | Yes | Windows native adapter and distinct-file DACL roundtrip test pass on HEAD 7e7b1e04; full descriptor/SACL semantic equivalence is not yet established |
| Darwin 0x0004 | Yes (basic structure) | Yes | macOS native adapter and distinct-file UUID/ACE roundtrip and legacy compatibility tests pass on HEAD 7e7b1e04; complete native semantic equivalence is not yet established |
| Extended NFSv4.1 0x0005 | Yes (XDR structural) | Yes | Not implemented |
| Unknown format ID | Preserved opaque | Yes | Not applied |

**Validation is not authorization equivalence.** The proof of binary roundtrip does not establish that all supported OS ACLs can be extracted losslessly, installed successfully, or have identical effective access checks.

## CI evidence and known gaps (2026-11-22)

- Verified HEAD: `7e7b1e046c6ee318d966653f60e8281b3a27d84f`. All 21 [test jobs (run 44994672019)](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44994672019), both [format/documentation jobs (run 44994671991)](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44994671991), all 12 [WASM jobs (run 44994671990)](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44994671990), and the [FreeBSD native job (run 44994671997)](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44994671997) succeeded. This is CI evidence for this specific HEAD, not a proof of complete ACL semantic coverage.
- [FreeBSD native job 137123024010](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44994671997/job/137123024010) created separate UFS filesystems for NFSv4 (`nfsv4acls`) and POSIX.1e (`tunefs -a enable`). Logs show `native_freebsd_nfs4_archive_roundtrip ... ok` and `native_freebsd_posix_directory_archive_roundtrip ... ok`, with 4 passed and 0 failed. These tests exercised real ACL capture → PNA → restore to distinct objects → recapture; they did not skip due to unsupported filesystems.
- [macOS stable job 137123024562](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44994672019/job/137123024562) executed `native_acl_file_roundtrip_with_uuid_principal ... ok`. The test restores to a distinct file and compares the captured UUID/ACE representation; NOACL/empty/absent and broader rights/flag combinations remain untested.
- [Windows stable job 137123024643](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44994672019/job/137123024643) executed `native_windows_file_dacl_roundtrip ... ok`. It does not test full owner/group-free self-relative descriptor validity, SACL acquisition, absent/NULL/empty ACL distinctions, or effective access checks.
- [Linux stable job 137123024556](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44994672019/job/137123024556) executed `create_archive_extract_and_restore_native_acl ... ok`. This checks a named numeric user ACL on a regular file; named groups, directory default ACLs, `fMOd` ordering and symlink/hardlink behavior require further native tests.
- `libpna` tests cover all five draft format IDs and unknown private IDs through an independent archive fixture, and reject some malformed/duplicate ACL data. Reserved-bit boundaries, post-FDAT editing semantics, solid/encrypted entries, unsafe-to-copy behavior, and effective authorization equivalence remain outside the proven scope.
- Earlier [run 44971120589](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44971120589) had a macOS nightly timestamp-test failure unrelated to ACL tests; the latest HEAD's macOS nightly job passed. The earlier failure's root cause has not been established; do not silently classify it as an ACL failure or a harmless flake.
- **Do not treat a green CI matrix as a claim of lossless ACL semantics for all native ACL variants.** Unsupported filesystem skips, structural-only checks and earlier HEADs cannot substitute for native roundtrip evidence.

## Important unresolved draft issues

1. **Windows:** confirm owner/group-free self-relative descriptor validity, exact distinction of absent/NULL/empty ACL, DACL-only/SACL-only coverage controls, full-SACL acquisition semantics (including integrity labels), and inherited control bits with native roundtrips.
2. **Darwin:** confirm native external encoding alignment/bit masks, UUID principal type semantics, `NOACL` sentinel with `acl_flags`, all types and raw filesystem-private flags; native owner UUID chunks are a separate proposal.
3. **NFSv4/FreeBSD:** verify UUID/name/UID mapping and ACE flag set, especially inherited ACE, mode bits, dacl/sacl applicability and possible loss of unknown principal data; FreeBSD NFSv4 and POSIX.1e basic native archive roundtrips pass on separate UFS filesystems, but full ACE/principal/inheritance coverage, ZFS NFSv4, and further POSIX edge cases are still missing.
4. **POSIX:** validate default ACL behavior on directories, numeric-ID and optional-name facets, symlinks/hardlinks, ACL vs `fMOd` ordering and consistency, and case where an ACL is absent but `fMOd` remains.
5. **Common/editor:** verify duplicate aCLS error handling without losing file contents, unsafe-to-copy rules when critical chunks are edited, unknown public/private IDs, preservation of after-FDAT placement, and encrypted/solid entries.
6. **CLI:** `pna acl get/set` still uses experimental legacy `faCl`/`faCe` and does not edit new `aCLS`. Model conversion is deliberately avoided. Non-Linux native adapters exist but remain experimental and unverified.

Do not merge into `main` before cross-platform tests and the draft's reserved-bit/validity rules are finalized.
