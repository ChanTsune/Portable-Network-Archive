#![cfg(not(target_family = "wasm"))]
//! TDD red-phase tests pinning bsdtar's observed behavior for the combination
//! of `-u` (update) and `--strip-components`.
//!
//! Each `#[test]` encodes a scenario empirically verified against bsdtar 3.5.3
//! / libarchive 3.5.3 (see investigation report). bsdtar's output is the
//! authoritative expectation; divergences from `pna compat bsdtar` represent
//! compatibility gaps to be addressed.
//!
//! ## Backend switch
//!
//! These tests exercise `pna compat bsdtar` by default. Setting the env var
//! `BSDTAR_REFERENCE=1` (optionally with `BSDTAR_PATH=/path/to/bsdtar`) swaps
//! the harness to invoke real `bsdtar` instead, producing a tar archive and
//! reading it via `bsdtar -tf`. This is a fixture-correctness check: the
//! reference run is expected to be GREEN — if it fails, the test fixture
//! itself encodes wrong expectations rather than a real pna gap.
//!
//! Mtime assertions are unconditional under the pna backend but skipped under
//! the bsdtar backend (since `bsdtar -tf` does not yield epoch-precision mtime
//! and reading the tar binary directly is out of scope for fixture-checking).

use crate::utils::{archive::for_each_entry, setup};
use assert_cmd::Command as AssertCommand;
use assert_cmd::cargo::cargo_bin_cmd;
use pna::Duration as PnaDuration;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command as StdCommand,
    time::{Duration, SystemTime},
};

const MTIME_2020: i64 = 1_577_836_800; // 2020-01-01T00:00:00Z
const MTIME_2026: i64 = 1_767_225_600; // 2026-01-01T00:00:00Z

fn is_bsdtar_reference() -> bool {
    std::env::var("BSDTAR_REFERENCE").is_ok_and(|value| value == "1")
}

fn bsdtar_binary() -> String {
    std::env::var("BSDTAR_PATH").unwrap_or_else(|_| "/usr/bin/bsdtar".into())
}

fn compat_cmd() -> AssertCommand {
    if is_bsdtar_reference() {
        AssertCommand::new(bsdtar_binary())
    } else {
        cargo_bin_cmd!("pna")
    }
}

fn compat_args<const N: usize>(rest: [&str; N]) -> Vec<&str> {
    let mut v: Vec<&str> = if is_bsdtar_reference() {
        Vec::new()
    } else {
        vec!["--quiet", "compat", "bsdtar", "--unstable"]
    };
    v.extend_from_slice(&rest);
    v
}

fn archive_path(base: &Path) -> PathBuf {
    base.join(if is_bsdtar_reference() {
        "archive.tar"
    } else {
        "archive.pna"
    })
}

fn set_mtime(path: impl AsRef<Path>, secs: i64) {
    let t = SystemTime::UNIX_EPOCH + Duration::from_secs(secs as u64);
    filetime::set_file_mtime(path, filetime::FileTime::from_system_time(t)).unwrap();
}

fn pna_dur(secs: i64) -> PnaDuration {
    PnaDuration::seconds(secs)
}

/// Build the standard `src/` fixture rooted at `base`.
/// Layout: `src/`, `src/sub/`, `src/A.txt`, `src/sub/B.txt` — all with mtime=2020.
fn build_src_fixture(base: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let src = base.join("src");
    let sub = src.join("sub");
    fs::create_dir_all(&sub).unwrap();
    let a = src.join("A.txt");
    let b = sub.join("B.txt");
    fs::write(&a, b"old A").unwrap();
    fs::write(&b, b"old B").unwrap();
    set_mtime(&a, MTIME_2020);
    set_mtime(&b, MTIME_2020);
    set_mtime(&sub, MTIME_2020);
    set_mtime(&src, MTIME_2020);
    (src, a, b)
}

fn collect_paths_with_mtime(archive: &Path) -> Vec<(String, Option<PnaDuration>)> {
    if is_bsdtar_reference() {
        // Use real bsdtar to list entries; mtime is reported as None because the
        // text format of `bsdtar -tf` is not directly comparable as epoch
        // seconds. mtime assertions are gated by `is_bsdtar_reference()`.
        let out = StdCommand::new(bsdtar_binary())
            .args(["-tf", archive.to_str().unwrap()])
            .output()
            .expect("bsdtar -tf failed to spawn");
        assert!(
            out.status.success(),
            "bsdtar -tf failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout)
            .expect("bsdtar -tf produced non-utf8 output")
            .lines()
            .map(|l| (l.to_string(), None))
            .collect()
    } else {
        let mut out = Vec::new();
        for_each_entry(archive, |e| {
            // pna's `EntryName::Display` does not append a trailing `/` to
            // directory entries, while bsdtar's `-tf` output does. To make
            // the two backends comparable, normalize to bsdtar's convention
            // by suffixing `/` for `DataKind::Directory` entries.
            let mut p = e.header().path().to_string();
            if matches!(e.header().data_kind(), pna::DataKind::Directory) && !p.ends_with('/') {
                p.push('/');
            }
            let m = e.metadata().modified();
            out.push((p, m));
        })
        .unwrap();
        out
    }
}

fn count_per_path(entries: &[(String, Option<PnaDuration>)]) -> HashMap<String, usize> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for (p, _) in entries {
        *counts.entry(p.clone()).or_insert(0) += 1;
    }
    counts
}

/// Precondition: An archive holds entries whose pathnames retain the leading
/// directory (e.g. `src/A.txt`). One disk entry under that directory is bumped
/// to a newer mtime; the rest match the archive's recorded mtime exactly.
/// Action: Run `pna compat bsdtar -u --strip-components=1 src/`.
/// Expectation: Pre-existing archive entries remain untouched. The newer disk
/// file is appended with its leading component removed. The directory `src/sub`
/// is also re-appended (without leading prefix) due to bsdtar's directory
/// trailing-slash quirk where directory paths fail to match the time-exclusion
/// tree. Entries whose disk mtime equals the archive mtime AND whose disk path
/// matches the archive path verbatim (e.g. `src/sub/B.txt`) are excluded and
/// not re-added.
#[test]
#[ignore = "directory trailing-slash quirk in archive vs disk path comparison — tracked in #3013"]
fn update_with_strip_components_writes_stripped_path() {
    setup();
    let base = Path::new("update_strip_writes_stripped");
    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(base).unwrap();

    let (src, a, _b) = build_src_fixture(base);
    let _ = src;
    let archive = archive_path(base);

    let archive_str = archive.to_str().unwrap();
    let base_str = base.to_str().unwrap();
    compat_cmd()
        .args(compat_args(["-cf", archive_str, "-C", base_str, "src/"]))
        .assert()
        .success();

    fs::write(&a, b"new A").unwrap();
    set_mtime(&a, MTIME_2026);
    set_mtime(base.join("src/sub"), MTIME_2020);
    set_mtime(base.join("src"), MTIME_2020);

    compat_cmd()
        .args(compat_args([
            "-uf",
            archive_str,
            "--strip-components",
            "1",
            "-C",
            base_str,
            "src/",
        ]))
        .assert()
        .success();

    // bsdtar 3.5.3 §3.3 observation:
    //   originals (4): src/, src/sub/, src/A.txt(2020), src/sub/B.txt(2020)
    //   appended (2):  sub/(2020), A.txt(2026)
    //   src/sub/B.txt is NOT re-added — disk path matches archive key, mtime EQUAL.
    let entries = collect_paths_with_mtime(&archive);
    let counts = count_per_path(&entries);
    for original in ["src/", "src/sub/", "src/A.txt", "src/sub/B.txt"] {
        assert_eq!(
            counts.get(original).copied().unwrap_or(0),
            1,
            "pre-existing entry `{}` must remain exactly once",
            original
        );
    }
    assert_eq!(
        counts.get("sub/").copied().unwrap_or(0),
        1,
        "stripped `sub/` directory entry must be appended"
    );
    assert_eq!(
        counts.get("A.txt").copied().unwrap_or(0),
        1,
        "stripped `A.txt` file entry must be appended"
    );
    assert_eq!(
        entries.len(),
        6,
        "expected exactly 6 entries; got {entries:?}"
    );

    if !is_bsdtar_reference() {
        let stripped_a_mtime = entries.iter().find(|(p, _)| p == "A.txt").map(|(_, m)| *m);
        assert_eq!(
            stripped_a_mtime,
            Some(Some(pna_dur(MTIME_2026))),
            "stripped `A.txt` should carry the disk's 2026 mtime"
        );
        let kept_src_a_mtime = entries
            .iter()
            .find(|(p, _)| p == "src/A.txt")
            .map(|(_, m)| *m);
        assert_eq!(
            kept_src_a_mtime,
            Some(Some(pna_dur(MTIME_2020))),
            "pre-existing `src/A.txt` must keep its 2020 mtime (strip is write-only)"
        );
    }
}

/// Precondition: An archive contains entries already in stripped form
/// (`A.txt`(2026), `sub/`(2020), `sub/B.txt`(2020)). The disk-side `src/`
/// directory carries OLDER copies (all mtime=2020) of the same logical files
/// but under a `src/` prefix.
/// Action: Run `pna compat bsdtar -u --strip-components=1 src/`.
/// Expectation: bsdtar's time-exclusion tree is keyed by the archive's stored
/// pathnames (`A.txt` etc.), but the disk reader yields strip-PRE pathnames
/// (`src/A.txt` etc.). The keys never align, so the time check is bypassed and
/// every disk entry is appended verbatim — even though the disk content is
/// older than the archive. The result has duplicate entries; on extraction
/// the later (older) copy silently overwrites the earlier (newer) copy.
/// This is bsdtar's well-known "broken" behavior pinned here for compatibility.
#[test]
fn update_with_strip_components_creates_duplicates_when_paths_misalign() {
    setup();
    let base = Path::new("update_strip_dup_misalign");
    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(base).unwrap();

    let (src, a, _b) = build_src_fixture(base);
    set_mtime(&a, MTIME_2026); // archive will record A.txt as 2026
    let archive = archive_path(base);

    let archive_str = archive.to_str().unwrap();
    let base_str = base.to_str().unwrap();
    let src_str = src.to_str().unwrap();
    compat_cmd()
        .args(compat_args([
            "-cf",
            archive_str,
            "-C",
            src_str,
            "A.txt",
            "sub/",
        ]))
        .assert()
        .success();

    // Now revert disk-side to all-2020 (older than the archive's A.txt=2026).
    set_mtime(&a, MTIME_2020);
    set_mtime(src.join("sub").join("B.txt"), MTIME_2020);
    set_mtime(src.join("sub"), MTIME_2020);
    set_mtime(&src, MTIME_2020);

    compat_cmd()
        .args(compat_args([
            "-uf",
            archive_str,
            "--strip-components",
            "1",
            "-C",
            base_str,
            "src/",
        ]))
        .assert()
        .success();

    // bsdtar 3.5.3 §3.4 observation (CRITICAL):
    //   originals (3): A.txt(2026), sub/(2020), sub/B.txt(2020)
    //   appended (3): sub/(2020), A.txt(2020), sub/B.txt(2020)  -- all duplicated
    let entries = collect_paths_with_mtime(&archive);
    let counts = count_per_path(&entries);
    assert_eq!(
        counts.get("A.txt").copied().unwrap_or(0),
        2,
        "`A.txt` must be duplicated; bsdtar pins this 'broken' append-without-mtime-check behavior"
    );
    assert_eq!(
        counts.get("sub/").copied().unwrap_or(0),
        2,
        "`sub/` must be duplicated"
    );
    assert_eq!(
        counts.get("sub/B.txt").copied().unwrap_or(0),
        2,
        "`sub/B.txt` must be duplicated"
    );
    assert_eq!(
        entries.len(),
        6,
        "exactly 6 entries (3 originals + 3 duplicates); got {entries:?}"
    );

    if !is_bsdtar_reference() {
        // Pin the mtime ordering: the original 2026 copy comes first, the 2020
        // duplicate is appended afterward. On extraction the second wins,
        // meaning the older content silently overwrites the newer one.
        let a_mtimes: Vec<Option<PnaDuration>> = entries
            .iter()
            .filter(|(p, _)| p == "A.txt")
            .map(|(_, m)| *m)
            .collect();
        assert_eq!(
            a_mtimes,
            vec![Some(pna_dur(MTIME_2026)), Some(pna_dur(MTIME_2020))],
            "first `A.txt` is the original (2026); second is the appended duplicate (2020). \
             This ordering means extraction loses the newer content — bsdtar's silent overwrite \
             is the pinned expected behavior."
        );
    }
}

/// Precondition: An archive holds `src/`-prefixed entries. The disk side has a
/// top-level file `top.txt` (1 component) and the existing `src/` tree.
/// Action: Run `pna compat bsdtar -u --strip-components=2 top.txt src/`.
/// Expectation: For each disk entry whose component count is less than the
/// strip count (or exactly equal — yielding an empty pathname), bsdtar's
/// `strip_components()` returns NULL and `edit_pathname` returns 1, causing
/// the entry to be silently dropped with no warning or error. The remaining
/// entry (`src/sub/B.txt`, 3 components → `B.txt` after strip) is matched
/// against the time-exclusion tree by its strip-PRE path `src/sub/B.txt`,
/// which aligns with the archive's stored key, and its mtime is EQUAL — so
/// it is excluded too. Net result: archive unchanged, exit 0.
#[test]
fn update_with_strip_components_silently_skips_short_paths() {
    setup();
    let base = Path::new("update_strip_silent_skip");
    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(base).unwrap();

    let (src, _a, _b) = build_src_fixture(base);
    let _ = src;
    let archive = archive_path(base);

    let archive_str = archive.to_str().unwrap();
    let base_str = base.to_str().unwrap();
    compat_cmd()
        .args(compat_args(["-cf", archive_str, "-C", base_str, "src/"]))
        .assert()
        .success();

    let top = base.join("top.txt");
    fs::write(&top, b"top").unwrap();
    set_mtime(&top, MTIME_2026);

    compat_cmd()
        .args(compat_args([
            "-uf",
            archive_str,
            "--strip-components",
            "2",
            "-C",
            base_str,
            "top.txt",
            "src/",
        ]))
        .assert()
        .success();

    // bsdtar 3.5.3 §3.5 observation: archive remains exactly the original 4
    // entries; no addition, no error.
    let entries = collect_paths_with_mtime(&archive);
    let counts = count_per_path(&entries);
    for original in ["src/", "src/sub/", "src/A.txt", "src/sub/B.txt"] {
        assert_eq!(
            counts.get(original).copied().unwrap_or(0),
            1,
            "pre-existing entry `{}` must remain exactly once",
            original
        );
    }
    assert_eq!(
        entries.len(),
        4,
        "archive must remain at 4 entries (every disk path is silently skipped); got {entries:?}"
    );
}

/// Precondition: An archive holds `src/`-prefixed entries. The disk has a
/// modified `src/A.txt` (2026); the rest match.
/// Action: Run `pna compat bsdtar -u -s ',^src,RENAMED,' --strip-components=1 src/`.
/// Expectation: Per `edit_pathname()` in libarchive (`tar/util.c:476`), the
/// transformations apply in order: (1) `-s` substitution rewrites `src/A.txt`
/// to `RENAMED/A.txt`, (2) `--strip-components=1` then strips one component,
/// yielding `A.txt`. The final archive must contain `A.txt`, never
/// `RENAMED/A.txt` and never `RENAMED`.
#[test]
#[ignore = "directory trailing-slash quirk in archive vs disk path comparison — tracked in #3013"]
fn update_with_substitution_then_strip_components() {
    setup();
    let base = Path::new("update_subst_then_strip");
    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(base).unwrap();

    let (src, a, _b) = build_src_fixture(base);
    let _ = src;
    let archive = archive_path(base);

    let archive_str = archive.to_str().unwrap();
    let base_str = base.to_str().unwrap();
    compat_cmd()
        .args(compat_args(["-cf", archive_str, "-C", base_str, "src/"]))
        .assert()
        .success();

    fs::write(&a, b"new A").unwrap();
    set_mtime(&a, MTIME_2026);
    set_mtime(base.join("src/sub"), MTIME_2020);
    set_mtime(base.join("src"), MTIME_2020);

    compat_cmd()
        .args(compat_args([
            "-uf",
            archive_str,
            "-s",
            ",^src,RENAMED,",
            "--strip-components",
            "1",
            "-C",
            base_str,
            "src/",
        ]))
        .assert()
        .success();

    // bsdtar 3.5.3 §3.6 observation: -s applies before strip-components.
    let entries = collect_paths_with_mtime(&archive);
    let counts = count_per_path(&entries);
    assert_eq!(
        counts.get("A.txt").copied().unwrap_or(0),
        1,
        "fully-stripped `A.txt` must be appended"
    );
    assert_eq!(
        counts.get("sub/").copied().unwrap_or(0),
        1,
        "fully-stripped `sub/` must be appended"
    );
    let renamed_paths: Vec<&String> = entries
        .iter()
        .map(|(p, _)| p)
        .filter(|p| p.starts_with("RENAMED"))
        .collect();
    assert!(
        renamed_paths.is_empty(),
        "no entry path may retain the `RENAMED` prefix; -s must run BEFORE strip-components. \
         found: {:?}",
        renamed_paths
    );

    if !is_bsdtar_reference() {
        let appended_a_mtime = entries.iter().find(|(p, _)| p == "A.txt").map(|(_, m)| *m);
        assert_eq!(
            appended_a_mtime,
            Some(Some(pna_dur(MTIME_2026))),
            "appended `A.txt` carries the disk's 2026 mtime"
        );
    }
}

/// Precondition: An archive contains stripped-form entries (`A.txt`(2026),
/// `sub/`(2020), `sub/B.txt`(2020)). The disk has a flat directory whose
/// entries match those archive paths exactly, with OLDER mtime (2020).
/// Action: Run `pna compat bsdtar -u -C flat A.txt B.txt` (no strip).
/// Expectation: Without `--strip-components`, the disk paths align with the
/// archive's exclusion-tree keys: disk `A.txt` (2020) hits archive `A.txt`
/// (2026) → OLDER → excluded → not re-added; disk `B.txt` (2020) does NOT hit
/// archive `sub/B.txt` → not excluded → appended. This is the control case
/// proving that mtime-based dedup works correctly when paths align (and
/// implicitly that the broken case in `creates_duplicates_when_paths_misalign`
/// is specifically caused by `--strip-components` defeating the alignment).
#[test]
fn update_with_aligned_paths_skips_unmodified() {
    setup();
    let base = Path::new("update_aligned_skip");
    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(base).unwrap();

    let (src, a, _b) = build_src_fixture(base);
    set_mtime(&a, MTIME_2026);
    let archive = archive_path(base);

    let archive_str = archive.to_str().unwrap();
    let src_str = src.to_str().unwrap();
    compat_cmd()
        .args(compat_args([
            "-cf",
            archive_str,
            "-C",
            src_str,
            "A.txt",
            "sub/",
        ]))
        .assert()
        .success();

    let flat = base.join("flat");
    fs::create_dir_all(&flat).unwrap();
    let flat_a = flat.join("A.txt");
    let flat_b = flat.join("B.txt");
    fs::write(&flat_a, b"old flat A").unwrap();
    fs::write(&flat_b, b"old flat B").unwrap();
    set_mtime(&flat_a, MTIME_2020);
    set_mtime(&flat_b, MTIME_2020);

    let flat_str = flat.to_str().unwrap();
    compat_cmd()
        .args(compat_args([
            "-uf",
            archive_str,
            "-C",
            flat_str,
            "A.txt",
            "B.txt",
        ]))
        .assert()
        .success();

    // bsdtar 3.5.3 §3.7 observation:
    //   final 4 entries: A.txt(2026 kept), sub/(2020), sub/B.txt(2020), B.txt(2020 added).
    let entries = collect_paths_with_mtime(&archive);
    let counts = count_per_path(&entries);
    assert_eq!(
        counts.get("A.txt").copied().unwrap_or(0),
        1,
        "`A.txt` must NOT be duplicated — disk path matches archive key, mtime OLDER → excluded"
    );
    assert_eq!(counts.get("sub/").copied().unwrap_or(0), 1);
    assert_eq!(counts.get("sub/B.txt").copied().unwrap_or(0), 1);
    assert_eq!(
        counts.get("B.txt").copied().unwrap_or(0),
        1,
        "`B.txt` must be appended — disk path does NOT match any archive key"
    );
    assert_eq!(
        entries.len(),
        4,
        "expected exactly 4 entries; got {entries:?}"
    );

    if !is_bsdtar_reference() {
        let a_mtime = entries.iter().find(|(p, _)| p == "A.txt").map(|(_, m)| *m);
        assert_eq!(
            a_mtime,
            Some(Some(pna_dur(MTIME_2026))),
            "`A.txt` mtime is the original 2026 (the disk's older 2020 was correctly rejected)"
        );
    }
}

/// Precondition: Same fixture as `update_with_strip_components_writes_stripped_path`
/// — archive holds `src/`-prefixed entries; one disk file is newer.
/// Action: Run `pna compat bsdtar -u --strip-components=1 src/`.
/// Expectation: This test isolates one invariant from the prior scenario: the
/// `--strip-components` transformation is applied ONLY at write time. Existing
/// archive entries must NOT be renamed in-place; they remain under their
/// originally-stored names with their original mtimes. New entries written by
/// this update are the only ones with stripped names.
#[test]
fn update_with_strip_components_preserves_existing_archive_entries() {
    setup();
    let base = Path::new("update_strip_preserve_existing");
    let _ = fs::remove_dir_all(base);
    fs::create_dir_all(base).unwrap();

    let (src, a, _b) = build_src_fixture(base);
    let _ = src;
    let archive = archive_path(base);

    let archive_str = archive.to_str().unwrap();
    let base_str = base.to_str().unwrap();
    compat_cmd()
        .args(compat_args(["-cf", archive_str, "-C", base_str, "src/"]))
        .assert()
        .success();

    fs::write(&a, b"new A").unwrap();
    set_mtime(&a, MTIME_2026);
    set_mtime(base.join("src/sub"), MTIME_2020);
    set_mtime(base.join("src"), MTIME_2020);

    compat_cmd()
        .args(compat_args([
            "-uf",
            archive_str,
            "--strip-components",
            "1",
            "-C",
            base_str,
            "src/",
        ]))
        .assert()
        .success();

    // bsdtar 3.5.3 invariant pinned here: strip is write-only. The four
    // pre-existing entries (`src/`, `src/sub/`, `src/A.txt`, `src/sub/B.txt`)
    // must all remain in the archive under their original names with their
    // original mtimes. They must not be transformed into `sub/`, `A.txt`, etc.
    let entries = collect_paths_with_mtime(&archive);
    let counts = count_per_path(&entries);
    for original in ["src/", "src/sub/", "src/A.txt", "src/sub/B.txt"] {
        assert_eq!(
            counts.get(original).copied().unwrap_or(0),
            1,
            "pre-existing entry `{}` must remain in the archive",
            original
        );
    }

    if !is_bsdtar_reference() {
        let kept_src_a_mtime = entries
            .iter()
            .find(|(p, _)| p == "src/A.txt")
            .map(|(_, m)| *m);
        assert_eq!(
            kept_src_a_mtime,
            Some(Some(pna_dur(MTIME_2020))),
            "pre-existing `src/A.txt` keeps its 2020 mtime (the disk's 2026 must NOT leak in)"
        );

        let kept_src_sub_b_mtime = entries
            .iter()
            .find(|(p, _)| p == "src/sub/B.txt")
            .map(|(_, m)| *m);
        assert_eq!(
            kept_src_sub_b_mtime,
            Some(Some(pna_dur(MTIME_2020))),
            "pre-existing `src/sub/B.txt` keeps its 2020 mtime"
        );
    }
}
