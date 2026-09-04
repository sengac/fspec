//! BUG-168: session-scoped model capability registry.
//!
//! The sessions layer resolves the active model's vision capability at session
//! creation and on every model switch, and stores it here. Tools (currently
//! the Read tool) consult the registry to decide the default PDF mode:
//!
//! - entry **absent**  -> historical visual default (unknown sessions)
//! - entry **present** -> `false` triggers the text fallback, `true` keeps visual
//!
//! PROV-144 adds a sibling per-session *image budget* registry (max images a
//! single Read tool result may return). It is resolved from the active
//! profile's `maxImages` by the sessions layer and consulted by the Read tool
//! (`read_image_budget`):
//!
//! - entry **absent** -> the tool layer applies its default of 4
//! - entry **`0`**    -> no-vision profile: image reads fail
//! - entry **`n >= 1`** -> a single Read result may return at most `n` images
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

/// PROV-144: the per-session image budget (max images a single Read tool
/// result may return). Sibling of `SESSION_MODEL_VISION` — populated at the
/// same set-sites and cleared at the same destroy site, so the two registries
/// can never drift relative to each other.
static SESSION_MODEL_MAX_IMAGES: once_cell::sync::Lazy<RwLock<HashMap<Uuid, u32>>> =
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

/// PROV-144: store the resolved image budget for a session.
///
/// Called by the sessions layer at session creation and on every model
/// switch, alongside [`set_session_model_vision`]. `Some(n)` stores the
/// profile's `maxImages` — `0` marks a no-vision profile (the Read tool fails
/// image reads), `n >= 1` caps a single Read result at `n` images. `None`
/// removes the entry (non-profile session) so the Read tool applies its
/// default budget of 4.
pub fn set_session_model_max_images(session_id: Uuid, max_images: Option<u32>) {
    let mut guard = match SESSION_MODEL_MAX_IMAGES.write() {
        Ok(g) => g,
        Err(_) => {
            tracing::warn!("PROV-144: max-images registry poisoned; budget not updated");
            return;
        }
    };
    match max_images {
        Some(n) => {
            guard.insert(session_id, n);
        }
        None => {
            guard.remove(&session_id);
        }
    }
}

/// PROV-144: the session's image budget.
///
/// Returns `None` when no entry is registered (non-profile session, or a
/// session created before PROV-144 / already destroyed) — the Read tool then
/// applies its default budget of 4.
pub fn session_model_max_images(session_id: Uuid) -> Option<u32> {
    SESSION_MODEL_MAX_IMAGES
        .read()
        .ok()
        .and_then(|guard| guard.get(&session_id).copied())
}

/// PROV-144: remove the max-images entry for one session (session destroy).
///
/// Cleared alongside the vision entry at the destroy set-site.
pub fn clear_session_model_max_images(session_id: Uuid) {
    if let Ok(mut guard) = SESSION_MODEL_MAX_IMAGES.write() {
        guard.remove(&session_id);
    }
}

/// Remove all entries (test teardown / process shutdown).
///
/// Clears BOTH the vision and the PROV-144 max-images registries so a
/// poisoned-lock reset or test teardown never leaves a stale budget behind.
pub fn clear_all_model_capabilities() {
    if let Ok(mut guard) = SESSION_MODEL_VISION.write() {
        guard.clear();
    }
    if let Ok(mut guard) = SESSION_MODEL_MAX_IMAGES.write() {
        guard.clear();
    }
}
