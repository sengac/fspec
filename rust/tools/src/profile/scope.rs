//! ProfileScope — RAII guard emitted by the `profile_scope!()` macro
//!
//! Feature: spec/features/agent-manager-profile-action.feature

use crate::profile::registry::ProfileRegistry;
use std::time::Instant;

/// RAII guard — on Drop, records elapsed time and increments the per-label counter.
pub struct ProfileScope {
    /// Fully-qualified label (produced by `concat!(module_path!(), "::", user_label)`)
    pub label: &'static str,
    /// Captured start instant
    pub start: Instant,
}

impl ProfileScope {
    /// Enter a new scope. Called from the `profile_scope!()` macro only when `PROFILING_ACTIVE`
    /// is true; the macro's surrounding `if` branch handles the inactive case.
    pub fn enter(label: &'static str) -> Option<Self> {
        ProfileRegistry::instance().record_enter(label);
        Some(Self {
            label,
            start: Instant::now(),
        })
    }
}

impl Drop for ProfileScope {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().as_nanos() as u64;
        ProfileRegistry::instance().record_exit(self.label, elapsed);
    }
}

/// Primary instrumentation marker macro.
///
/// Expands to a branch-gated `ProfileScope::enter` call. When `PROFILING_ACTIVE == false`,
/// the compiled code is one Relaxed atomic load + branch-not-taken (~1 ns on aarch64).
#[macro_export]
macro_rules! profile_scope {
    ($label:literal) => {
        let _profile_guard = if $crate::profile::registry::PROFILING_ACTIVE
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            $crate::profile::scope::ProfileScope::enter(concat!(module_path!(), "::", $label))
        } else {
            None
        };
    };
}
