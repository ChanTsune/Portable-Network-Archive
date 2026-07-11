use crate::{
    cli::{GlobalContext, PasswordArgs},
    command::{
        Command, ask_password,
        core::{collect_split_archives, run_read_entries},
    },
};
use clap::{Parser, ValueHint};
use pna::{Encryption, NormalEntry, ReadEntry, ReadOptions, SolidEntry};
use std::{io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(
    after_long_help = "Note: for encrypted entries, a wrong password is indistinguishable from corruption."
)]
pub(crate) struct VerifyCommand {
    #[arg(short = 'f', long = "file", help = "Archive file path", value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(
        long,
        help = "Verify chunk structure and CRC32 only, without decoding entry data",
        long_help = "Verify chunk structure and CRC32 only, without decoding entry data. Solid blocks are still decoded because enumerating their entries requires decompression and decryption, so stream corruption inside a solid block is detected even with --fast. Encrypted normal entries are counted as ok because nothing is decoded, so the skipped category does not apply."
    )]
    fast: bool,
    #[command(flatten)]
    password: PasswordArgs,
}

impl Command for VerifyCommand {
    #[inline]
    fn execute(self, _: &GlobalContext) -> anyhow::Result<()> {
        verify_archive(self)
    }
}

#[derive(Default)]
struct VerifyReport {
    ok: usize,
    failed: usize,
    skipped: usize,
    encrypted_failure: bool,
}

impl VerifyReport {
    fn total(&self) -> usize {
        self.ok + self.failed + self.skipped
    }
}

fn verify_archive(args: VerifyCommand) -> anyhow::Result<()> {
    let fast = args.fast;
    let password = ask_password(args.password)?;
    let password = password.as_deref();
    let read_options = ReadOptions::with_password(password);
    let archives = collect_split_archives(&args.archive)?;
    let mut report = VerifyReport::default();
    let mut solid_blocks = 0usize;
    let mut resyncing = false;
    let result = run_read_entries(
        archives,
        |entry| {
            match entry {
                Err(err) if err.kind() == io::ErrorKind::InvalidData => {
                    // A broken chunk aborts the current entry's assembly, and
                    // the remainder of that entry surfaces as one or more
                    // InvalidData errors before the iterator resyncs on the
                    // next entry header. Count the corrupted entry once;
                    // adjacent corrupted entries with no healthy entry in
                    // between collapse into a single failure (accepted
                    // approximation).
                    if !resyncing {
                        println!("<corrupted entry>: FAILED ({err})");
                        report.failed += 1;
                        resyncing = true;
                    }
                    Ok(())
                }
                Err(err) => Err(err),
                Ok(read_entry) => {
                    resyncing = false;
                    match read_entry {
                        ReadEntry::Solid(solid) => {
                            solid_blocks += 1;
                            if solid.encryption() != Encryption::No && password.is_none() {
                                println!("<solid block #{solid_blocks}>: skipped (encrypted)");
                                report.skipped += 1;
                                return Ok(());
                            }
                            if let Err(err) =
                                verify_solid(&solid, password, &read_options, fast, &mut report)
                            {
                                println!("<solid block #{solid_blocks}>: FAILED ({err})");
                                report.failed += 1;
                                report.encrypted_failure |= solid.encryption() != Encryption::No;
                            }
                        }
                        ReadEntry::Normal(entry) => {
                            verify_entry(&entry, password, &read_options, fast, &mut report)
                        }
                    }
                    Ok(())
                }
            }
        },
        false,
    );
    print_summary(&report);
    if let Err(err) = result {
        return Err(
            anyhow::Error::new(err).context("archive structure is broken; verification aborted")
        );
    }
    if report.failed > 0 {
        anyhow::bail!(
            "verification failed: {} of {} entries are corrupted",
            report.failed,
            report.total()
        );
    }
    Ok(())
}

fn verify_solid(
    solid: &SolidEntry,
    password: Option<&[u8]>,
    read_options: &ReadOptions,
    fast: bool,
    report: &mut VerifyReport,
) -> io::Result<()> {
    for entry in solid.entries(read_options)? {
        verify_entry(&entry?, password, read_options, fast, report);
    }
    Ok(())
}

fn verify_entry(
    entry: &NormalEntry,
    password: Option<&[u8]>,
    read_options: &ReadOptions,
    fast: bool,
    report: &mut VerifyReport,
) {
    if fast {
        // Entry assembly already validated chunk CRC32 and structure.
        report.ok += 1;
        return;
    }
    let encrypted = entry.header().encryption() != Encryption::No;
    if encrypted && password.is_none() {
        // Decoding is impossible without the password.
        report.skipped += 1;
        return;
    }
    match read_through(entry, read_options) {
        Ok(size) => {
            if let Some(hint) = entry.metadata().raw_file_size()
                && hint != u128::from(size)
            {
                log::warn!(
                    "{}: size hint (fSIZ) mismatch: recorded {hint}, actual {size}",
                    entry.name()
                );
            }
            log::debug!("{}: ok", entry.name());
            report.ok += 1;
        }
        Err(err) => {
            println!("{}: FAILED ({err})", entry.name());
            report.failed += 1;
            report.encrypted_failure |= encrypted;
        }
    }
}

fn read_through(entry: &NormalEntry, read_options: &ReadOptions) -> io::Result<u64> {
    let mut reader = entry.reader(read_options)?;
    io::copy(&mut reader, &mut io::sink())
}

fn print_summary(report: &VerifyReport) {
    if report.skipped > 0 {
        println!(
            "{} entries skipped (encrypted; no password provided)",
            report.skipped
        );
    }
    println!(
        "total: {}, ok: {}, failed: {}, skipped (encrypted): {}",
        report.total(),
        report.ok,
        report.failed,
        report.skipped
    );
    if report.encrypted_failure {
        println!(
            "note: a wrong password is indistinguishable from corruption for encrypted entries"
        );
    }
}
