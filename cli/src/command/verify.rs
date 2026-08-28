use crate::{
    cli::{GlobalContext, PasswordArgs},
    command::{
        Command, ask_password,
        core::{collect_split_archives, run_read_entries},
    },
};
use clap::{Parser, ValueHint};
use pna::{CipherMode, Encryption, NormalEntry, ReadEntry, ReadOptions, SolidEntry};
use std::{io, path::PathBuf};

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
#[command(
    after_long_help = "Note: for entries encrypted in CBC or CTR mode, a wrong password is indistinguishable from corruption. GCM mode instead reports a key mismatch, meaning the password does not match the key derivation parameters recorded for the entry: either the password is wrong, or those recorded parameters were themselves altered."
)]
pub(crate) struct VerifyCommand {
    #[arg(short = 'f', long = "file", help = "Archive file path", value_hint = ValueHint::FilePath)]
    archive: PathBuf,
    #[arg(
        long,
        help = "Verify chunk structure and CRC32 only, without decoding entry data",
        long_help = "Verify chunk structure and CRC32 only. No entry data is decoded, so neither the entries contained in a solid block nor corruption that leaves a chunk's CRC32 intact are checked. Every entry whose chunks are intact is counted as ok, and no password is required."
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
    /// Gates the note that a wrong password cannot be told from corruption.
    unauthenticated_failure: bool,
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
            // A chunk that fails its CRC32 arrives as `Err(InvalidData)`, so an `Ok`
            // entry has passed every chunk's CRC32.
            match entry {
                Err(err) if err.kind() == io::ErrorKind::InvalidData => {
                    // One broken entry surfaces as several of these before the
                    // iterator resyncs on the next entry header, so only the
                    // first is counted; adjacent corrupted entries collapse
                    // into a single failure (accepted approximation).
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
                    if fast {
                        report.ok += 1;
                        return Ok(());
                    }
                    match read_entry {
                        ReadEntry::Solid(solid) => {
                            solid_blocks += 1;
                            if solid.encryption() != Encryption::NO && password.is_none() {
                                println!("<solid block #{solid_blocks}>: skipped (encrypted)");
                                report.skipped += 1;
                                return Ok(());
                            }
                            if let Err(err) =
                                verify_solid(&solid, password, &read_options, &mut report)
                            {
                                println!("<solid block #{solid_blocks}>: FAILED ({err})");
                                report.failed += 1;
                                report.unauthenticated_failure |=
                                    is_unauthenticated(solid.encryption(), solid.cipher_mode());
                            }
                        }
                        ReadEntry::Normal(entry) => {
                            verify_entry(&entry, password, &read_options, &mut report)
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
    report: &mut VerifyReport,
) -> io::Result<()> {
    for entry in solid.entries(read_options)? {
        verify_entry(&entry?, password, read_options, report);
    }
    Ok(())
}

fn verify_entry(
    entry: &NormalEntry,
    password: Option<&[u8]>,
    read_options: &ReadOptions,
    report: &mut VerifyReport,
) {
    let encrypted = entry.header().encryption() != Encryption::NO;
    if encrypted && password.is_none() {
        // Decoding is impossible without the password.
        report.skipped += 1;
        return;
    }
    match read_through(entry, read_options) {
        Ok(size) => {
            let source_size = entry
                .sparse_map()
                .map_or(u128::from(size), |map| u128::from(map.logical_size()));
            if let Some(hint) = entry.metadata().raw_file_size()
                && hint != source_size
            {
                log::warn!(
                    "{}: size hint (fSIZ) mismatch: recorded {hint}, actual {source_size}",
                    entry.name()
                );
            }
            log::debug!("{}: ok", entry.name());
            report.ok += 1;
        }
        Err(err) => {
            println!("{}: FAILED ({err})", entry.name());
            report.failed += 1;
            report.unauthenticated_failure |=
                is_unauthenticated(entry.header().encryption(), entry.header().cipher_mode());
        }
    }
}

/// Whether the encryption offers confidentiality without authenticity, so a
/// decoding failure cannot tell a wrong password from corruption.
///
/// Both fields are allow-lists. An entry whose encryption method or cipher mode
/// this build does not implement fails for a reason a password cannot explain,
/// so the note must not be attributed to it.
fn is_unauthenticated(encryption: Encryption, cipher_mode: CipherMode) -> bool {
    matches!(encryption, Encryption::AES | Encryption::CAMELLIA)
        && matches!(cipher_mode, CipherMode::CBC | CipherMode::CTR)
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
    if report.unauthenticated_failure {
        println!(
            "note: a wrong password is indistinguishable from corruption for CBC/CTR encrypted entries"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cbc_and_ctr_encrypted_entries_are_unauthenticated() {
        assert!(is_unauthenticated(Encryption::AES, CipherMode::CBC));
        assert!(is_unauthenticated(Encryption::AES, CipherMode::CTR));
        assert!(is_unauthenticated(Encryption::CAMELLIA, CipherMode::CBC));
        assert!(is_unauthenticated(Encryption::CAMELLIA, CipherMode::CTR));
    }

    #[test]
    fn gcm_encrypted_entries_are_authenticated() {
        assert!(!is_unauthenticated(Encryption::AES, CipherMode::GCM));
        assert!(!is_unauthenticated(Encryption::CAMELLIA, CipherMode::GCM));
    }

    #[test]
    fn unencrypted_entries_are_not_unauthenticated() {
        assert!(!is_unauthenticated(Encryption::NO, CipherMode::CBC));
        assert!(!is_unauthenticated(Encryption::NO, CipherMode::GCM));
    }

    #[test]
    fn unsupported_encryption_method_is_not_unauthenticated() {
        assert!(!is_unauthenticated(
            Encryption::from_byte(5),
            CipherMode::CBC
        ));
    }

    #[test]
    fn unsupported_cipher_mode_is_not_unauthenticated() {
        assert!(!is_unauthenticated(
            Encryption::AES,
            CipherMode::from_byte(3)
        ));
    }
}
