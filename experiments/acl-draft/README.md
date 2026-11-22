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
| POSIX.1e 0x0001 | Yes | Yes | Linux native adapter and FreeBSD POSIX adapter; dedicated FreeBSD POSIX.1e native CI test added but not yet verified |
| Extended NFSv4 0x0002 | Yes (XDR structural) | Yes | FreeBSD native adapter and distinct-file archive roundtrip passed on a dedicated UFS filesystem with nfsv4acls; broader ACE semantics unverified |
| Windows NT 0x0003 | Yes (basic descriptor structure) | Yes | Windows native adapter and distinct-file DACL roundtrip test pass on prior HEAD; full descriptor/SACL semantic equivalence is not yet established |
| Darwin 0x0004 | Yes (basic structure) | Yes | macOS native adapter and legacy compatibility tests pass on prior HEAD; complete native semantic equivalence is not yet established |
| Extended NFSv4.1 0x0005 | Yes (XDR structural) | Yes | Not implemented |
| Unknown format ID | Preserved opaque | Yes | Not applied |

**Validation is not authorization equivalence.** The proof of binary roundtrip does not establish that all supported OS ACLs can be extracted losslessly, installed successfully, or have identical effective access checks.

## CI evidence and known gaps (2026-11-22)

- At HEAD `805570f90788de907da70d1690b5013c2a9cf87e`, the FreeBSD POSIX.1e dedicated UFS workflow and native distinct-directory roundtrip test have been added; the new CI runs are pending. The previous HEAD `6a29a79a6d852e9d7ce7bc99769734ecd5187d59` passed [test](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44981082041), [format/docs](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44981082049), [WASM](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44981082037), and [FreeBSD](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44981082046). These prior-HEAD results are not evidence for the new POSIX test.

- Inspected branch HEAD before this documentation update: `c0da73fbb9fa60ea25908834037825764bb0fd7e`. [FreeBSD run 44971678812](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44971678812) passed, including actionlint and native ACL tests. [Actions lint run 44971678893](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44971678893) passed. There is no full Linux/macOS/Windows + format + WASM CI suite for this exact HEAD; earlier results must not be attributed to it.
- FreeBSD native evidence: [job 137052640594](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44971678812/job/137052640594) logs `tunefs: NFSv4 ACLs set` and `/dev/md0 on /tmp/pna-acl-nfs4 (ufs, local, soft-updates, nfsv4acls)`. With `PNA_REQUIRE_FREEBSD_NFS4_NATIVE=1`, `native_freebsd_nfs4_archive_roundtrip ... ok` executed (not skipped), and the three FreeBSD ACL unit tests passed. The NFSv4 test captures ACL from one file, stores it in a PNA archive, restores onto a distinct file and compares recaptured bytes. This is concrete native roundtrip evidence for the exercised ACL, **not** proof for every NFSv4 ACE, principal, inheritance or access-check case.
- The FreeBSD POSIX numeric-principal unit test constructs and decodes an ACL in memory. HEAD `805570f9` additionally introduces a separate POSIX.1e-capable UFS volume and a directory native access/default ACL → PNA → distinct-directory restoration test with numeric principal 65432. **The new test has not yet passed CI; do not count it as evidence until its job logs show execution and success.** The NFSv4 volume cannot substitute for the POSIX dialect. The dedicated FreeBSD workflow runs on each experiment-branch push.
- [HEAD `fc0df239` test run 44951056016](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44951056016) passed all 21 jobs (Linux, macOS and Windows stable/beta/nightly, Windows x64/ARM64); [format/docs 44951055913](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44951055913) and [WASM 44951055915](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44951055915) passed. These are earlier-HEAD regression results only.
- [HEAD `424b0f4a` test run 44971120589](https://github.com/ChanTsune/Portable-Network-Archive/actions/runs/44971120589) failed only the macOS nightly job (20/21 succeeded). The failed job reports two timestamp assertions in `bsdtar_create_timestamp_flags` and `bsdtar_append_mtime`; the native Darwin ACL unit tests passed. The timestamp failures require a separate root-cause investigation; do not claim they are ACL failures or assume they are harmless flakes.
- Windows distinct-file DACL capture → PNA → restoration → recapture passes in the earlier test matrix. The test does not prove the full DACL/SACL/owner/group-free descriptor validity matrix, effective authorization equivalence, or SACL category coverage. The native setter fallback also requires explicit verification of which API succeeded.
- Do not count a skipped test, a structural-only comparison, or a previously green commit as proof of full native roundtrip.

## Important unresolved draft issues

1. **Windows:** confirm owner/group-free self-relative descriptor validity, exact distinction of absent/NULL/empty ACL, DACL-only/SACL-only coverage controls, full-SACL acquisition semantics (including integrity labels), and inherited control bits with native roundtrips.
2. **Darwin:** confirm native external encoding alignment/bit masks, UUID principal type semantics, `NOACL` sentinel with `acl_flags`, all types and raw filesystem-private flags; native owner UUID chunks are a separate proposal.
3. **NFSv4/FreeBSD:** verify UUID/name/UID mapping and ACE flag set, especially inherited ACE, mode bits, dacl/sacl applicability and possible loss of unknown principal data; FreeBSD NFSv4 basic native archive roundtrip now passes on UFS with nfsv4acls, but full ACE/principal/inheritance coverage and POSIX.1e real-filesystem validation are still missing.
4. **POSIX:** validate default ACL behavior on directories, numeric-ID and optional-name facets, symlinks/hardlinks, ACL vs `fMOd` ordering and consistency, and case where an ACL is absent but `fMOd` remains.
5. **Common/editor:** verify duplicate aCLS error handling without losing file contents, unsafe-to-copy rules when critical chunks are edited, unknown public/private IDs, preservation of after-FDAT placement, and encrypted/solid entries.
6. **CLI:** `pna acl get/set` still uses experimental legacy `faCl`/`faCe` and does not edit new `aCLS`. Model conversion is deliberately avoided. Non-Linux native adapters exist but remain experimental and unverified.

Do not merge into `main` before cross-platform tests and the draft's reserved-bit/validity rules are finalized.
