use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Condvar, Mutex},
};

struct PathOrderState {
    current_seq: Mutex<usize>,
    condvar: Condvar,
}

struct PathRegistration {
    next_seq: usize,
    state: Arc<PathOrderState>,
}

/// Per-path ordered serialization for concurrent archive extraction.
///
/// Assigns monotonically increasing sequence numbers per path during sequential
/// archive iteration, then enforces that order when parallel workers call
/// [`PathOrderTicket::wait_for_turn`]. Registration and worker execution are
/// interleaved — workers may begin before all entries have been registered.
///
/// Paths that appear only once (the common case) pass through [`wait_for_turn`]
/// without blocking.
#[derive(Default)]
pub(crate) struct OrderedPathLocks {
    registry: Mutex<HashMap<PathBuf, PathRegistration>>,
}

impl std::fmt::Debug for OrderedPathLocks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderedPathLocks").finish_non_exhaustive()
    }
}

/// A ticket granting ordered access to a specific path.
///
/// Created by [`OrderedPathLocks::register`] during sequential iteration,
/// then consumed by [`wait_for_turn`](PathOrderTicket::wait_for_turn) in a parallel worker.
///
/// If dropped without calling `wait_for_turn`, the sequence counter is
/// automatically advanced to prevent deadlocking subsequent entries.
#[must_use = "ticket must be consumed via wait_for_turn() to avoid delaying other entries"]
pub(crate) struct PathOrderTicket {
    seq: usize,
    state: Option<Arc<PathOrderState>>,
}

/// RAII guard that advances the per-path sequence counter when dropped,
/// unblocking the next waiting entry for the same path.
pub(crate) struct PathOrderGuard(Arc<PathOrderState>, usize);

impl Drop for PathOrderTicket {
    fn drop(&mut self) {
        if let Some(state) = self.state.take() {
            advance_seq(&state, self.seq);
        }
    }
}

impl Drop for PathOrderGuard {
    fn drop(&mut self) {
        advance_seq(&self.0, self.1);
    }
}

/// Waits until `current_seq >= seq`, then sets it to `seq + 1`.
/// Uses poison-recovery to prevent double-panic during unwinding.
fn advance_seq(state: &PathOrderState, seq: usize) {
    let mut current = state.current_seq.lock().unwrap_or_else(|e| e.into_inner());
    while *current < seq {
        current = state
            .condvar
            .wait(current)
            .unwrap_or_else(|e| e.into_inner());
    }
    *current = seq + 1;
    state.condvar.notify_all();
}

impl OrderedPathLocks {
    /// Registers an upcoming write to `path` and returns a ticket with the assigned
    /// sequence number.
    ///
    /// Must be called from the sequential iteration thread. Sequence numbers increase
    /// monotonically per path (0, 1, 2, …).
    pub(crate) fn register(&self, path: &Path) -> PathOrderTicket {
        let mut map = self
            .registry
            .lock()
            .expect("path lock registry mutex poisoned");
        let reg = map
            .entry(path.to_path_buf())
            .or_insert_with(|| PathRegistration {
                next_seq: 0,
                state: Arc::new(PathOrderState {
                    current_seq: Mutex::new(0),
                    condvar: Condvar::new(),
                }),
            });
        let seq = reg.next_seq;
        reg.next_seq += 1;
        PathOrderTicket {
            seq,
            state: Some(Arc::clone(&reg.state)),
        }
    }
}

impl PathOrderTicket {
    /// Blocks until it is this entry's turn for its path, then returns a guard
    /// that advances the counter when dropped.
    ///
    /// For the first (or only) entry on a path, this returns immediately.
    pub(crate) fn wait_for_turn(mut self) -> PathOrderGuard {
        let state = self
            .state
            .take()
            .expect("wait_for_turn called on consumed ticket");
        let mut current = state.current_seq.lock().unwrap_or_else(|e| e.into_inner());
        while *current != self.seq {
            current = state
                .condvar
                .wait(current)
                .unwrap_or_else(|e| e.into_inner());
        }
        drop(current);
        PathOrderGuard(state, self.seq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn single_path_no_contention() {
        let locks = OrderedPathLocks::default();
        let ticket = locks.register(Path::new("a.txt"));
        let _guard = ticket.wait_for_turn();
    }

    #[test]
    fn independent_paths_no_blocking() {
        let locks = OrderedPathLocks::default();
        let t1 = locks.register(Path::new("a.txt"));
        let t2 = locks.register(Path::new("b.txt"));
        let _g2 = t2.wait_for_turn();
        let _g1 = t1.wait_for_turn();
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn duplicate_path_ordering() {
        let locks = Arc::new(OrderedPathLocks::default());
        let order = Arc::new(AtomicUsize::new(0));

        let t0 = locks.register(Path::new("file.txt"));
        let t1 = locks.register(Path::new("file.txt"));
        let t2 = locks.register(Path::new("file.txt"));

        std::thread::scope(|s| {
            let order_clone = Arc::clone(&order);
            s.spawn(move || {
                let _g = t0.wait_for_turn();
                assert_eq!(order_clone.fetch_add(1, Ordering::SeqCst), 0);
            });
            let order_clone = Arc::clone(&order);
            s.spawn(move || {
                let _g = t1.wait_for_turn();
                assert_eq!(order_clone.fetch_add(1, Ordering::SeqCst), 1);
            });
            let order_clone = Arc::clone(&order);
            s.spawn(move || {
                let _g = t2.wait_for_turn();
                assert_eq!(order_clone.fetch_add(1, Ordering::SeqCst), 2);
            });
        });
        assert_eq!(order.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn guard_advances_on_drop() {
        let locks = OrderedPathLocks::default();
        let t0 = locks.register(Path::new("x"));
        let t1 = locks.register(Path::new("x"));

        let guard = t0.wait_for_turn();
        drop(guard);
        let _g1 = t1.wait_for_turn();
    }

    #[test]
    fn dropped_ticket_does_not_block_successor() {
        let locks = OrderedPathLocks::default();
        let t0 = locks.register(Path::new("x"));
        let t1 = locks.register(Path::new("x"));

        drop(t0);
        let _g1 = t1.wait_for_turn();
    }
}
