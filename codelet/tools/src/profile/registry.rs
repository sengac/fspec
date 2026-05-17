//! ProfileRegistry — singleton storage for per-scope metrics during an active profile window
//!
//! Feature: spec/features/agent-manager-profile-action.feature
//!
//! The registry holds a `DashMap<&'static str, ScopeMetrics>` keyed by compile-time static
//! label strings, and the critical `PROFILING_ACTIVE` atomic gate that makes the entire
//! instrumentation layer cost sub-1ns when nobody is profiling.

use crate::profile::result::ScopeReport;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Global gate: when `false`, `profile_scope!()` expands to one Relaxed load + branch-not-taken.
/// When `true`, `ProfileScope` RAII guards accumulate timing and counter data into the registry.
pub static PROFILING_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Per-scope atomic counters updated from the hot path via Relaxed ops.
#[derive(Debug, Default)]
pub struct ScopeMetrics {
    /// Total number of `ProfileScope::enter` calls for this label
    pub call_count: AtomicU64,
    /// Sum of self-time in nanoseconds across all completed guards
    pub total_self_ns: AtomicU64,
    /// Largest single-iteration self-time observed in nanoseconds
    pub max_iter_ns: AtomicU64,
    /// Last time this scope was entered (unix millis)
    pub last_seen_unix_ms: AtomicU64,
    /// Number of guards still in flight (incremented on enter, decremented on drop)
    pub currently_executing: AtomicI64,
}

impl ScopeMetrics {
    fn reset(&self) {
        self.call_count.store(0, Ordering::Relaxed);
        self.total_self_ns.store(0, Ordering::Relaxed);
        self.max_iter_ns.store(0, Ordering::Relaxed);
        self.last_seen_unix_ms.store(0, Ordering::Relaxed);
        self.currently_executing.store(0, Ordering::Relaxed);
    }
}

/// Singleton registry holding per-label `ScopeMetrics` during a profile window.
pub struct ProfileRegistry {
    scopes: DashMap<&'static str, ScopeMetrics>,
}

static REGISTRY: Lazy<ProfileRegistry> = Lazy::new(|| ProfileRegistry {
    scopes: DashMap::new(),
});

impl ProfileRegistry {
    /// Get the global registry instance.
    pub fn instance() -> &'static ProfileRegistry {
        &REGISTRY
    }

    /// Record that a new guard was entered for `label`. Called by `ProfileScope::enter`.
    pub fn record_enter(&self, label: &'static str) {
        let entry = self.scopes.entry(label).or_default();
        entry.call_count.fetch_add(1, Ordering::Relaxed);
        entry.currently_executing.fetch_add(1, Ordering::Relaxed);
        entry
            .last_seen_unix_ms
            .store(now_unix_ms(), Ordering::Relaxed);
    }

    /// Record that a guard finished for `label`, accumulating elapsed nanos. Called on Drop.
    pub fn record_exit(&self, label: &'static str, elapsed_ns: u64) {
        if let Some(entry) = self.scopes.get(label) {
            entry
                .total_self_ns
                .fetch_add(elapsed_ns, Ordering::Relaxed);
            entry.currently_executing.fetch_sub(1, Ordering::Relaxed);
            let prev_max = entry.max_iter_ns.load(Ordering::Relaxed);
            if elapsed_ns > prev_max {
                // Best-effort update; races are acceptable for a max counter
                entry
                    .max_iter_ns
                    .store(elapsed_ns.max(prev_max), Ordering::Relaxed);
            }
        }
    }

    /// Zero all per-scope counters at the start of a profile session.
    pub fn reset_all(&self) {
        for entry in self.scopes.iter() {
            entry.value().reset();
        }
    }

    /// Build a sorted & filtered `ScopeReport` list.
    pub fn snapshot_scopes(
        &self,
        label_prefix: Option<&str>,
        top_n: usize,
        sort_by_self_ms: bool,
        duration_secs: u32,
    ) -> Vec<ScopeReport> {
        let mut entries: Vec<ScopeReport> = self
            .scopes
            .iter()
            .filter_map(|entry| {
                let label = *entry.key();
                if let Some(prefix) = label_prefix {
                    if !label.starts_with(prefix) {
                        return None;
                    }
                }
                let metrics = entry.value();
                let call_count = metrics.call_count.load(Ordering::Relaxed);
                if call_count == 0 {
                    return None;
                }
                let total_self_ns = metrics.total_self_ns.load(Ordering::Relaxed);
                let max_iter_ns = metrics.max_iter_ns.load(Ordering::Relaxed);
                let currently_executing =
                    metrics.currently_executing.load(Ordering::Relaxed) as i32;
                let total_self_ms = total_self_ns as f64 / 1_000_000.0;
                let max_iter_ms = max_iter_ns as f64 / 1_000_000.0;
                let calls_per_sec = if duration_secs == 0 {
                    0.0
                } else {
                    call_count as f64 / duration_secs as f64
                };
                Some(ScopeReport {
                    label: label.to_string(),
                    call_count,
                    total_self_ms,
                    max_iter_ms,
                    calls_per_sec,
                    currently_executing_at_end: currently_executing,
                })
            })
            .collect();

        if sort_by_self_ms {
            entries.sort_by(|a, b| {
                b.total_self_ms
                    .partial_cmp(&a.total_self_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        } else {
            entries.sort_by_key(|b| std::cmp::Reverse(b.call_count));
        }

        entries.truncate(top_n);
        entries
    }

    /// Total number of scopes currently registered.
    pub fn scope_count(&self) -> usize {
        self.scopes.len()
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
