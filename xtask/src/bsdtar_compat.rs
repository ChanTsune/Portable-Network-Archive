use clap::Parser;
use std::collections::BTreeMap;
use std::os::unix::fs::{self as unix_fs, MetadataExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
use std::{fs, io};

// ---------------------------------------------------------------------------
// Axis 1: Pre-existing filesystem state at extraction target
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreExisting {
    None,
    RegularFile,
    Directory,
    SymlinkToFile,
    SymlinkToDir,
    HardLink,
}

impl PreExisting {
    const ALL: &[Self] = &[
        Self::None,
        Self::RegularFile,
        Self::Directory,
        Self::SymlinkToFile,
        Self::SymlinkToDir,
        Self::HardLink,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::RegularFile => "File",
            Self::Directory => "Dir",
            Self::SymlinkToFile => "SymFile",
            Self::SymlinkToDir => "SymDir",
            Self::HardLink => "HLink",
        }
    }
}

// ---------------------------------------------------------------------------
// Axis 2: Archive entry type being extracted
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArchiveEntryType {
    File,
    Directory,
    Symlink,
    HardLink,
    NestedPath,
}

impl ArchiveEntryType {
    const ALL: &[Self] = &[
        Self::File,
        Self::Directory,
        Self::Symlink,
        Self::HardLink,
        Self::NestedPath,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Directory => "Dir",
            Self::Symlink => "Sym",
            Self::HardLink => "HLink",
            Self::NestedPath => "Nested",
        }
    }
}

// ---------------------------------------------------------------------------
// Axis 3: Extract options
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverwriteMode {
    Default,
    KeepOldFiles,
    KeepNewerFiles,
}

impl OverwriteMode {
    const ALL: &[Self] = &[Self::Default, Self::KeepOldFiles, Self::KeepNewerFiles];

    fn label(self) -> &'static str {
        match self {
            Self::Default => "ow",
            Self::KeepOldFiles => "keep_old",
            Self::KeepNewerFiles => "keep_newer",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExtractOptions {
    overwrite_mode: OverwriteMode,
    unlink_first: bool,
    absolute_paths: bool,
}

// ---------------------------------------------------------------------------
// Sub-axis: mtime relationship (only for KeepNewerFiles + pre-existing)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MtimeRelation {
    Irrelevant,
    ArchiveNewer,
    ArchiveOlder,
}

// ---------------------------------------------------------------------------
// Generated scenario
// ---------------------------------------------------------------------------

struct GeneratedScenario {
    name: String,
    pre_existing: PreExisting,
    entry_type: ArchiveEntryType,
    options: ExtractOptions,
    mtime_relation: MtimeRelation,
}

fn build_scenario_name(
    pre: PreExisting,
    entry: ArchiveEntryType,
    opts: &ExtractOptions,
    mtime: MtimeRelation,
) -> String {
    let mut name = format!(
        "{}_over_{}_{}",
        entry.label(),
        pre.label(),
        opts.overwrite_mode.label()
    );
    if opts.unlink_first {
        name.push_str("_U");
    }
    if opts.absolute_paths {
        name.push_str("_P");
    }
    match mtime {
        MtimeRelation::Irrelevant => {}
        MtimeRelation::ArchiveNewer => name.push_str("_arc_newer"),
        MtimeRelation::ArchiveOlder => name.push_str("_arc_older"),
    }
    name
}

fn generate_scenarios() -> Vec<GeneratedScenario> {
    let mut scenarios = Vec::new();

    for &pre in PreExisting::ALL {
        for &entry in ArchiveEntryType::ALL {
            for &ow_mode in OverwriteMode::ALL {
                for unlink in [false, true] {
                    for abs_paths in [false, true] {
                        let options = ExtractOptions {
                            overwrite_mode: ow_mode,
                            unlink_first: unlink,
                            absolute_paths: abs_paths,
                        };

                        let mtime_variants = if ow_mode == OverwriteMode::KeepNewerFiles
                            && pre != PreExisting::None
                        {
                            &[MtimeRelation::ArchiveNewer, MtimeRelation::ArchiveOlder][..]
                        } else {
                            &[MtimeRelation::Irrelevant][..]
                        };

                        for &mtime_rel in mtime_variants {
                            let name = build_scenario_name(pre, entry, &options, mtime_rel);
                            scenarios.push(GeneratedScenario {
                                name,
                                pre_existing: pre,
                                entry_type: entry,
                                options,
                                mtime_relation: mtime_rel,
                            });
                        }
                    }
                }
            }
        }
    }
    scenarios
}

// ---------------------------------------------------------------------------
// Materializers: axis values → FileSpec lists
// ---------------------------------------------------------------------------

enum FileSpec {
    File {
        path: &'static str,
        contents: &'static [u8],
        mtime_epoch: Option<i64>,
    },
    Dir {
        path: &'static str,
        mtime_epoch: Option<i64>,
    },
    Symlink {
        path: &'static str,
        target: &'static str,
    },
    HardLink {
        path: &'static str,
        original: &'static str,
    },
}

fn epoch_to_system_time(epoch: i64) -> SystemTime {
    if epoch >= 0 {
        SystemTime::UNIX_EPOCH + Duration::from_secs(epoch as u64)
    } else {
        SystemTime::UNIX_EPOCH - Duration::from_secs(epoch.unsigned_abs())
    }
}

fn materialize(root: &Path, specs: &[FileSpec]) -> io::Result<()> {
    for spec in specs {
        match spec {
            FileSpec::File {
                path,
                contents,
                mtime_epoch,
            } => {
                let full = root.join(path);
                if let Some(parent) = full.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&full, contents)?;
                let epoch = mtime_epoch.unwrap_or(DEFAULT_MTIME);
                let time = epoch_to_system_time(epoch);
                let file = fs::File::options().write(true).open(&full)?;
                file.set_modified(time)?;
            }
            FileSpec::Dir { path, mtime_epoch } => {
                let full = root.join(path);
                fs::create_dir_all(&full)?;
                let epoch = mtime_epoch.unwrap_or(DEFAULT_MTIME);
                let time = epoch_to_system_time(epoch);
                let dir = fs::File::open(&full)?;
                dir.set_modified(time)?;
            }
            FileSpec::Symlink { path, target } => {
                let full = root.join(path);
                if let Some(parent) = full.parent() {
                    fs::create_dir_all(parent)?;
                }
                unix_fs::symlink(target, &full)?;
            }
            FileSpec::HardLink { path, original } => {
                let full = root.join(path);
                let orig = root.join(original);
                if let Some(parent) = full.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::hard_link(&orig, &full)?;
            }
        }
    }
    Ok(())
}

const ARCHIVE_MTIME: i64 = 2_000_000_000;
const EXISTING_MTIME: i64 = 1;
const ARCHIVE_MTIME_OLD: i64 = 1;
const EXISTING_MTIME_NEW: i64 = 2_000_000_000;
/// Fixed default mtime for deterministic comparison when mtime is irrelevant
const DEFAULT_MTIME: i64 = 1_500_000_000;

fn make_source_files(entry_type: ArchiveEntryType, mtime: MtimeRelation) -> Vec<FileSpec> {
    let mtime_epoch = match mtime {
        MtimeRelation::ArchiveNewer => Some(ARCHIVE_MTIME),
        MtimeRelation::ArchiveOlder => Some(ARCHIVE_MTIME_OLD),
        MtimeRelation::Irrelevant => None,
    };

    match entry_type {
        ArchiveEntryType::File => vec![FileSpec::File {
            path: "target",
            contents: b"from_archive",
            mtime_epoch,
        }],
        ArchiveEntryType::Directory => vec![
            FileSpec::Dir {
                path: "target",
                mtime_epoch,
            },
            FileSpec::File {
                path: "target/marker.txt",
                contents: b"inside_dir",
                mtime_epoch,
            },
        ],
        ArchiveEntryType::Symlink => vec![
            FileSpec::File {
                path: "symlink_dest",
                contents: b"symlink_target_content",
                mtime_epoch: None,
            },
            FileSpec::Symlink {
                path: "target",
                target: "symlink_dest",
            },
        ],
        ArchiveEntryType::HardLink => vec![
            FileSpec::File {
                path: "link_original",
                contents: b"hardlink_content",
                mtime_epoch,
            },
            FileSpec::HardLink {
                path: "target",
                original: "link_original",
            },
        ],
        ArchiveEntryType::NestedPath => vec![
            FileSpec::File {
                path: "target/sub/deep/file.txt",
                contents: b"deep_content",
                mtime_epoch,
            },
            FileSpec::File {
                path: "independent.txt",
                contents: b"independent_content",
                mtime_epoch: None,
            },
        ],
    }
}

fn make_pre_existing(pre: PreExisting, mtime: MtimeRelation) -> Vec<FileSpec> {
    let existing_mtime = match mtime {
        MtimeRelation::ArchiveNewer => Some(EXISTING_MTIME),
        MtimeRelation::ArchiveOlder => Some(EXISTING_MTIME_NEW),
        MtimeRelation::Irrelevant => None,
    };

    match pre {
        PreExisting::None => vec![],
        PreExisting::RegularFile => vec![FileSpec::File {
            path: "target",
            contents: b"existing_content",
            mtime_epoch: existing_mtime,
        }],
        PreExisting::Directory => vec![
            FileSpec::Dir {
                path: "target",
                mtime_epoch: existing_mtime,
            },
            FileSpec::File {
                path: "target/old_marker.txt",
                contents: b"was_here",
                mtime_epoch: existing_mtime,
            },
        ],
        PreExisting::SymlinkToFile => vec![
            FileSpec::File {
                path: "real_file",
                contents: b"real_file_content",
                mtime_epoch: existing_mtime,
            },
            FileSpec::Symlink {
                path: "target",
                target: "real_file",
            },
        ],
        PreExisting::SymlinkToDir => vec![
            FileSpec::Dir {
                path: "real_dir",
                mtime_epoch: existing_mtime,
            },
            FileSpec::Symlink {
                path: "target",
                target: "real_dir",
            },
        ],
        PreExisting::HardLink => vec![
            FileSpec::File {
                path: "target",
                contents: b"existing_content",
                mtime_epoch: existing_mtime,
            },
            FileSpec::HardLink {
                path: "target_link",
                original: "target",
            },
        ],
    }
}

fn make_extract_args(opts: &ExtractOptions) -> Vec<&'static str> {
    let mut args = Vec::new();
    match opts.overwrite_mode {
        OverwriteMode::Default => {}
        OverwriteMode::KeepOldFiles => args.push("-k"),
        OverwriteMode::KeepNewerFiles => args.push("--keep-newer-files"),
    }
    if opts.unlink_first {
        args.push("-U");
    }
    if opts.absolute_paths {
        args.push("-P");
    }
    args
}

// ---------------------------------------------------------------------------
// Snapshot infrastructure (kept from original)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
enum FsEntry {
    File {
        contents: Vec<u8>,
        mode: u32,
        mtime_secs: i64,
    },
    Dir {
        mode: u32,
    },
    Symlink {
        target: PathBuf,
    },
}

impl std::fmt::Display for FsEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FsEntry::File {
                contents,
                mode,
                mtime_secs,
            } => match std::str::from_utf8(contents) {
                Ok(s) => write!(f, "File({s:?}, mode={mode:04o}, mtime={mtime_secs})"),
                Err(_) => write!(
                    f,
                    "File({} bytes, mode={mode:04o}, mtime={mtime_secs})",
                    contents.len()
                ),
            },
            FsEntry::Dir { mode } => write!(f, "Dir(mode={mode:04o})"),
            FsEntry::Symlink { target } => write!(f, "Symlink({})", target.display()),
        }
    }
}

#[derive(Debug)]
struct FsSnapshot(BTreeMap<PathBuf, FsEntry>);

impl FsSnapshot {
    fn capture(root: &Path) -> io::Result<Self> {
        let mut entries = BTreeMap::new();
        Self::walk(root, root, &mut entries)?;
        Ok(Self(entries))
    }

    fn walk(root: &Path, dir: &Path, entries: &mut BTreeMap<PathBuf, FsEntry>) -> io::Result<()> {
        let mut dir_entries: Vec<_> = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
        dir_entries.sort_by_key(|e| e.file_name());

        for entry in dir_entries {
            let path = entry.path();
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            let meta = fs::symlink_metadata(&path)?;

            if meta.is_symlink() {
                let target = fs::read_link(&path)?;
                entries.insert(rel, FsEntry::Symlink { target });
            } else if meta.is_dir() {
                let mode = meta.mode() & 0o7777;
                entries.insert(rel.clone(), FsEntry::Dir { mode });
                Self::walk(root, &path, entries)?;
            } else {
                let contents = fs::read(&path)?;
                let mode = meta.mode() & 0o7777;
                let mtime_secs = meta.mtime();
                entries.insert(
                    rel,
                    FsEntry::File {
                        contents,
                        mode,
                        mtime_secs,
                    },
                );
            }
        }
        Ok(())
    }
}

struct Diff {
    path: PathBuf,
    bsdtar: Option<FsEntry>,
    pna: Option<FsEntry>,
}

fn compare_snapshots(bsdtar: &FsSnapshot, pna: &FsSnapshot) -> Vec<Diff> {
    let mut diffs = Vec::new();
    let all_keys: std::collections::BTreeSet<_> = bsdtar.0.keys().chain(pna.0.keys()).collect();

    for key in all_keys {
        let b = bsdtar.0.get(key);
        let p = pna.0.get(key);
        if b != p {
            diffs.push(Diff {
                path: key.clone(),
                bsdtar: b.cloned(),
                pna: p.cloned(),
            });
        }
    }
    diffs
}

// ---------------------------------------------------------------------------
// Command execution
// ---------------------------------------------------------------------------

fn run_cmd(cmd: &mut Command) -> Result<(), Box<dyn std::error::Error>> {
    let output = cmd.output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("command failed: {:?}\nstderr: {stderr}", cmd.get_program()).into());
    }
    Ok(())
}

struct CmdResult {
    success: bool,
}

fn run_cmd_capture(cmd: &mut Command) -> io::Result<CmdResult> {
    let output = cmd.output()?;
    Ok(CmdResult {
        success: output.status.success(),
    })
}

fn find_pna_binary() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let status = Command::new("cargo")
        .args(["build", "-p", "portable-network-archive"])
        .status()?;
    if !status.success() {
        return Err("failed to build pna binary".into());
    }
    let path = PathBuf::from("target/debug/pna");
    if !path.exists() {
        return Err(format!("pna binary not found at {}", path.display()).into());
    }
    Ok(fs::canonicalize(path)?)
}

fn check_bsdtar(bsdtar_bin: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Command::new(bsdtar_bin)
        .arg("--version")
        .output()
        .map_err(|_| format!("{} not found in PATH", bsdtar_bin.display()))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Scenario runner
// ---------------------------------------------------------------------------

enum ScenarioResult {
    Pass,
    Fail(Vec<Diff>),
    ExitMismatch { bsdtar_ok: bool, pna_ok: bool },
}

fn run_scenario(
    scenario: &GeneratedScenario,
    bsdtar_bin: &Path,
    pna_bin: &Path,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let source_files = make_source_files(scenario.entry_type, scenario.mtime_relation);
    let pre_existing = make_pre_existing(scenario.pre_existing, scenario.mtime_relation);
    let extract_args = make_extract_args(&scenario.options);

    let work = tempfile::tempdir()?;
    let work = work.path();

    // --- bsdtar side ---
    let bsdtar_src = work.join("bsdtar_src");
    let bsdtar_dst = work.join("bsdtar_dst");
    let bsdtar_archive = work.join("archive.tar");
    fs::create_dir_all(&bsdtar_src)?;
    fs::create_dir_all(&bsdtar_dst)?;

    materialize(&bsdtar_src, &source_files)?;
    materialize(&bsdtar_dst, &pre_existing)?;

    run_cmd(
        Command::new(bsdtar_bin)
            .args(["-cf", bsdtar_archive.to_str().unwrap()])
            .arg("-C")
            .arg(&bsdtar_src)
            .arg("."),
    )?;

    let bsdtar_result = run_cmd_capture(
        Command::new(bsdtar_bin)
            .args(["-xf", bsdtar_archive.to_str().unwrap()])
            .args(&extract_args)
            .arg("-C")
            .arg(&bsdtar_dst),
    )?;

    let bsdtar_snap = FsSnapshot::capture(&bsdtar_dst)?;

    // --- pna side ---
    let pna_src = work.join("pna_src");
    let pna_dst = work.join("pna_dst");
    let pna_archive = work.join("archive.pna");
    fs::create_dir_all(&pna_src)?;
    fs::create_dir_all(&pna_dst)?;

    materialize(&pna_src, &source_files)?;
    materialize(&pna_dst, &pre_existing)?;

    run_cmd(
        Command::new(pna_bin)
            .args(["experimental", "stdio", "--unstable"])
            .args(["-cf", pna_archive.to_str().unwrap()])
            .arg("-C")
            .arg(&pna_src)
            .arg("."),
    )?;

    let pna_result = run_cmd_capture(
        Command::new(pna_bin)
            .args(["experimental", "stdio", "--unstable"])
            .args(["-xf", pna_archive.to_str().unwrap()])
            .args(&extract_args)
            .arg("-C")
            .arg(&pna_dst),
    )?;

    let pna_snap = FsSnapshot::capture(&pna_dst)?;

    // --- Compare ---
    if bsdtar_result.success != pna_result.success {
        return Ok(ScenarioResult::ExitMismatch {
            bsdtar_ok: bsdtar_result.success,
            pna_ok: pna_result.success,
        });
    }

    let diffs = compare_snapshots(&bsdtar_snap, &pna_snap);
    if diffs.is_empty() {
        Ok(ScenarioResult::Pass)
    } else {
        Ok(ScenarioResult::Fail(diffs))
    }
}

// ---------------------------------------------------------------------------
// CLI entry point
// ---------------------------------------------------------------------------

#[derive(Parser)]
pub struct BsdtarCompatArgs {
    /// Path to a bsdtar-compatible binary to use as the reference oracle
    #[arg(long, default_value = "bsdtar")]
    pub bsdtar: PathBuf,

    /// Run only scenarios whose names contain this substring
    #[arg(long)]
    pub filter: Option<String>,
}

pub fn run(args: BsdtarCompatArgs) -> Result<(), Box<dyn std::error::Error>> {
    let bsdtar_bin = &args.bsdtar;
    check_bsdtar(bsdtar_bin)?;
    let pna_bin = find_pna_binary()?;

    let all_scenarios = generate_scenarios();
    let scenarios: Vec<_> = match &args.filter {
        Some(pattern) => all_scenarios
            .iter()
            .filter(|s| s.name.contains(pattern.as_str()))
            .collect(),
        None => all_scenarios.iter().collect(),
    };

    let total = scenarios.len();
    eprintln!(
        "bsdtar-compat: running {total} scenarios (of {} total)",
        all_scenarios.len()
    );

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut errors = 0usize;

    for scenario in &scenarios {
        match run_scenario(scenario, bsdtar_bin, &pna_bin) {
            Ok(ScenarioResult::Pass) => {
                eprintln!("[PASS] {}", scenario.name);
                passed += 1;
            }
            Ok(ScenarioResult::Fail(diffs)) => {
                eprintln!("[FAIL] {}", scenario.name);
                for diff in &diffs {
                    eprintln!("  diff at {}:", diff.path.display());
                    match &diff.bsdtar {
                        Some(e) => eprintln!("    bsdtar: {e}"),
                        None => eprintln!("    bsdtar: (absent)"),
                    }
                    match &diff.pna {
                        Some(e) => eprintln!("    pna:    {e}"),
                        None => eprintln!("    pna:    (absent)"),
                    }
                }
                failed += 1;
            }
            Ok(ScenarioResult::ExitMismatch { bsdtar_ok, pna_ok }) => {
                eprintln!(
                    "[FAIL] {} (exit mismatch: bsdtar={}, pna={})",
                    scenario.name,
                    if bsdtar_ok { "ok" } else { "error" },
                    if pna_ok { "ok" } else { "error" },
                );
                failed += 1;
            }
            Err(e) => {
                eprintln!("[ERROR] {}: {e}", scenario.name);
                errors += 1;
            }
        }
    }

    eprintln!("---");
    eprintln!("{total} scenarios: {passed} passed, {failed} failed, {errors} errors");

    if failed > 0 || errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}
