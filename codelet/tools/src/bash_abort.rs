//! Abort signal management for bash tool cancellation.
//!
//! Per-session abort flags (BUG-129) so that pressing ESC in one session
//! only cancels bash commands in that session, not others.

use crate::session_registry::SessionRegistry;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;

/// Per-session abort flags for bash tool cancellation.
/// Each session gets its own `AtomicBool` so that pressing ESC in one session
/// only aborts bash commands in that session.
static BASH_ABORT_FLAGS: Lazy<SessionRegistry<Arc<AtomicBool>>> = Lazy::new(SessionRegistry::new);

/// Set the abort flag for a specific session to request cancellation.
pub fn request_bash_abort(session_id: Uuid) {
    BASH_ABORT_FLAGS.with(&session_id, |flag| {
        flag.store(true, Ordering::Release);
    });
}

/// Clear the abort flag for a specific session (call before starting a new command).
/// Lazily inserts a fresh flag if none exists for this session.
pub fn clear_bash_abort(session_id: Uuid) {
    BASH_ABORT_FLAGS.get_or_insert_with(
        session_id,
        || Arc::new(AtomicBool::new(false)),
        |flag| flag.store(false, Ordering::Release),
    );
}

/// Check if abort has been requested for a specific session.
pub fn is_bash_abort_requested(session_id: Uuid) -> bool {
    BASH_ABORT_FLAGS
        .with(&session_id, |flag| flag.load(Ordering::Acquire))
        .unwrap_or(false)
}

/// Remove the abort flag entry for a session (call on session destroy to prevent leaks).
pub fn unregister_bash_abort_flag(session_id: Uuid) {
    BASH_ABORT_FLAGS.remove(&session_id);
}
