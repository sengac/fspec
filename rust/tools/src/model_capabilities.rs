//! BUG-168: session-scoped model capability registry.
//!
//! The sessions layer resolves the active model's vision capability at session
//! creation and on every model switch, and stores it here. Tools (currently
//! the Read tool) consult the registry to decide the default PDF mode:
//!
//! - entry **absent**  -> historical visual default (unknown sessions)
//! - entry **present** -> `false` triggers the text fallback, `true` keeps visual
//!
//! The registry follows the codelet-tools session-registry pattern used by
//! `done.rs` (armed/acceptance state) — a process-global `RwLock<HashMap<Uuid, _>>`
//! with graceful poison handling (a poisoned lock degrades to the absent-entry
//! behaviour rather than panicking).

use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

static SESSION_MODEL_VISION: once_cell::sync::Lazy<RwLock<HashMap<Uuid, bool>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Store the resolved vision capability for a session.
///
/// Called by the sessions layer at session creation and on every model switch.
pub fn set_session_model_vision(session_id: Uuid, supports_vision: bool) {
    if let Ok(mut guard) = SESSION_MODEL_VISION.write() {
        guard.insert(session_id, supports_vision);
    } else {
        tracing::warn!("BUG-168: model capability registry poisoned; capability not stored");
    }
}

/// Whether the session has a capability entry registered at all.
///
/// Absent entries mean the session was never plumbed (or was destroyed) and
/// must keep the historical visual default.
pub fn session_has_capabilities(session_id: Uuid) -> bool {
    SESSION_MODEL_VISION
        .read()
        .is_ok_and(|guard| guard.contains_key(&session_id))
}

/// Whether the session model supports vision (only meaningful when
/// [`session_has_capabilities`] is true).
pub fn session_supports_vision(session_id: Uuid) -> bool {
    SESSION_MODEL_VISION
        .read()
        .is_ok_and(|guard| guard.get(&session_id).copied().unwrap_or(false))
}

/// Remove the entry for one session (session destroy).
pub fn clear_session_model_vision(session_id: Uuid) {
    if let Ok(mut guard) = SESSION_MODEL_VISION.write() {
        guard.remove(&session_id);
    }
}

/// Remove all entries (test teardown / process shutdown).
pub fn clear_all_model_capabilities() {
    if let Ok(mut guard) = SESSION_MODEL_VISION.write() {
        guard.clear();
    }
}
