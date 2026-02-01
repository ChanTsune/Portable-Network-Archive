use flume::{Receiver, Sender};
use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
};

/// Sender for notifying write completion with result.
pub(crate) type CompletionSender = Sender<io::Result<()>>;

/// Receiver for waiting on write completion.
type CompletionReceiver = Receiver<io::Result<()>>;

/// Tracks in-progress path writes and ensures same-path writes are serialized.
///
/// Archives may contain multiple entries targeting the same path (e.g., a file
/// entry followed by metadata updates, or intentional duplicate entries where
/// later entries should overwrite earlier ones). This structure ensures writes
/// complete in archive order.
///
/// Before writing to a path, call [`begin_write`](Self::begin_write) which:
/// 1. Waits for any previous write to the same path to complete
/// 2. Returns a sender to notify when the new write completes
#[derive(Debug, Default)]
pub(crate) struct PendingPaths {
    pending: HashMap<PathBuf, CompletionReceiver>,
}

impl PendingPaths {
    /// Begins a write operation to the specified path.
    ///
    /// If a previous write to this path is in progress, blocks until it completes.
    /// Returns a sender that MUST be used to notify completion (success or failure).
    ///
    /// # Warning
    ///
    /// Dropping the sender without sending will cause subsequent writes to this path
    /// to fail with `BrokenPipe`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The previous write's completion channel was disconnected (task panic)
    /// - The previous write failed (propagates that error)
    pub(crate) fn begin_write(&mut self, path: &Path) -> io::Result<CompletionSender> {
        if let Some(rx) = self.pending.remove(path) {
            match rx.recv() {
                Ok(result) => result?,
                Err(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "previous write task did not send completion",
                    ));
                }
            }
        }

        let (tx, rx) = flume::bounded(1);
        self.pending.insert(path.to_path_buf(), rx);

        Ok(tx)
    }

    /// Waits for all pending writes to complete.
    ///
    /// Call this before returning from the extraction function to ensure
    /// all spawned tasks have finished.
    ///
    /// # Errors
    ///
    /// Returns the first error encountered from any pending write. All pending
    /// writes are still awaited even after an error is found, and subsequent
    /// errors are logged but not returned.
    pub(crate) fn wait_all(&mut self) -> io::Result<()> {
        let mut first_error: Option<io::Error> = None;
        for (path, rx) in self.pending.drain() {
            match rx.recv() {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    log::error!("Write to {} failed: {}", path.display(), e);
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                }
                Err(_) => {
                    log::error!("Write task for {} did not send completion", path.display());
                    if first_error.is_none() {
                        first_error = Some(io::Error::new(
                            io::ErrorKind::BrokenPipe,
                            "write task did not send completion",
                        ));
                    }
                }
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn begin_write_returns_sender() {
        let mut pending = PendingPaths::default();
        let path = Path::new("/tmp/test");

        let tx = pending.begin_write(path).unwrap();
        tx.send(Ok(())).unwrap();

        assert!(pending.wait_all().is_ok());
    }

    #[test]
    #[ignore]
    fn begin_write_waits_for_previous_completion() {
        let mut pending = PendingPaths::default();
        let path = Path::new("/tmp/test");

        let tx1 = pending.begin_write(path).unwrap();

        let tx2 = rayon::scope(|s| {
            s.spawn(|_| {
                thread::sleep(std::time::Duration::from_millis(50));
                tx1.send(Ok(())).unwrap();
            });
            pending.begin_write(path).unwrap()
        });

        tx2.send(Ok(())).unwrap();

        assert!(pending.wait_all().is_ok());
    }

    #[test]
    fn begin_write_propagates_previous_error() {
        let mut pending = PendingPaths::default();
        let path = Path::new("/tmp/test");

        let tx1 = pending.begin_write(path).unwrap();
        tx1.send(Err(io::Error::other("write failed"))).unwrap();

        let result = pending.begin_write(path);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::Other);
    }

    #[test]
    fn begin_write_returns_error_on_disconnected_channel() {
        let mut pending = PendingPaths::default();
        let path = Path::new("/tmp/test");

        let tx1 = pending.begin_write(path).unwrap();
        drop(tx1);

        let result = pending.begin_write(path);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    }

    #[test]
    fn wait_all_returns_first_error() {
        let mut pending = PendingPaths::default();

        let tx1 = pending.begin_write(Path::new("/tmp/a")).unwrap();
        let tx2 = pending.begin_write(Path::new("/tmp/b")).unwrap();

        tx1.send(Err(io::Error::new(io::ErrorKind::NotFound, "not found")))
            .unwrap();
        tx2.send(Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "denied",
        )))
        .unwrap();

        let result = pending.wait_all();
        assert!(result.is_err());
    }

    #[test]
    fn wait_all_handles_disconnected_channel() {
        let mut pending = PendingPaths::default();

        let tx = pending.begin_write(Path::new("/tmp/test")).unwrap();
        drop(tx);

        let result = pending.wait_all();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
    }
}
