//! Tracked channel wrappers — instrumented broadcast/mpsc channels for profile reporting
//!
//! Feature: spec/features/agent-manager-profile-action.feature

use crate::profile::result::ChannelReport;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Handle stored in the global registry for each live tracked channel.
#[derive(Clone)]
struct ChannelHandle {
    name: String,
    sender_count_fn: Arc<dyn Fn() -> u32 + Send + Sync>,
    receiver_count_fn: Arc<dyn Fn() -> u32 + Send + Sync>,
    queued_now_fn: Arc<dyn Fn() -> u64 + Send + Sync>,
    lagged_counter: Arc<AtomicU64>,
}

impl std::fmt::Debug for ChannelHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelHandle")
            .field("name", &self.name)
            .field("lagged", &self.lagged_counter.load(Ordering::Relaxed))
            .finish()
    }
}

/// Global registry of tracked channels. Enumerated by `ProfileSession::run()` at window end.
pub struct ChannelRegistry {
    channels: DashMap<String, ChannelHandle>,
}

static REGISTRY: Lazy<ChannelRegistry> = Lazy::new(|| ChannelRegistry {
    channels: DashMap::new(),
});

impl ChannelRegistry {
    /// Get the global channel registry instance.
    pub fn instance() -> &'static ChannelRegistry {
        &REGISTRY
    }

    /// Register a new tracked channel. Returns the shared lagged counter so the wrapper can
    /// increment it as receivers observe `RecvError::Lagged(n)`.
    fn register(&self, handle: ChannelHandle) -> Arc<AtomicU64> {
        let lagged = handle.lagged_counter.clone();
        self.channels.insert(handle.name.clone(), handle);
        lagged
    }

    /// Unregister a tracked channel on Drop.
    fn unregister(&self, name: &str) {
        self.channels.remove(name);
    }

    /// Reset all per-channel `lagged` counters at the start of a profile session.
    pub fn reset_lagged_counters(&self) {
        for entry in self.channels.iter() {
            entry.value().lagged_counter.store(0, Ordering::Relaxed);
        }
    }

    /// Capture a snapshot of every registered channel as `ChannelReport`s.
    pub fn snapshot(&self) -> Vec<ChannelReport> {
        self.channels
            .iter()
            .map(|entry| {
                let handle = entry.value();
                ChannelReport {
                    name: handle.name.clone(),
                    sender_count: (handle.sender_count_fn)(),
                    receiver_count: (handle.receiver_count_fn)(),
                    queued_at_end: (handle.queued_now_fn)(),
                    lagged_during_window: handle.lagged_counter.load(Ordering::Relaxed),
                }
            })
            .collect()
    }
}

/// Instrumented wrapper around `tokio::sync::broadcast::Sender<T>`.
///
/// On construction, registers under `name` in the global `ChannelRegistry`.
/// On Drop, unregisters.
pub struct TrackedBroadcast<T: Clone + Send + 'static> {
    /// Wrapped tokio broadcast sender (public so call sites can subscribe, send, etc.)
    pub sender: broadcast::Sender<T>,
    /// Stable channel name registered in `ChannelRegistry`
    pub name: String,
    /// Shared counter incremented when receivers observe `RecvError::Lagged(n)`
    pub lagged_counter: Arc<AtomicU64>,
}

impl<T: Clone + Send + 'static> TrackedBroadcast<T> {
    /// Create a new tracked broadcast channel with the given stable name and capacity.
    pub fn new(name: String, capacity: usize) -> (Self, broadcast::Receiver<T>) {
        let (tx, rx) = broadcast::channel::<T>(capacity);
        let lagged_counter = Arc::new(AtomicU64::new(0));
        let sender_for_rc = tx.clone();
        let sender_for_q = tx.clone();
        let handle = ChannelHandle {
            name: name.clone(),
            sender_count_fn: Arc::new(|| 1),
            receiver_count_fn: Arc::new(move || sender_for_rc.receiver_count() as u32),
            queued_now_fn: Arc::new(move || sender_for_q.len() as u64),
            lagged_counter: lagged_counter.clone(),
        };
        let _lagged_ref = ChannelRegistry::instance().register(handle);
        (
            Self {
                sender: tx,
                name,
                lagged_counter,
            },
            rx,
        )
    }

    /// Expose the underlying broadcast sender for direct use.
    pub fn sender(&self) -> &broadcast::Sender<T> {
        &self.sender
    }
}

impl<T: Clone + Send + 'static> Drop for TrackedBroadcast<T> {
    fn drop(&mut self) {
        ChannelRegistry::instance().unregister(&self.name);
    }
}
