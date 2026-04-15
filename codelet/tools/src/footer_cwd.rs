//! Per-session footer CWD tracking via SessionRegistry.
//!
//! Stores the "last known working directory" for each session so the
//! footer poller can display the actual directory commands are running in,
//! not just the static project root from session creation.
//!
//! Data flow:
//!   BashTool::call() → resolve_cwd() → update_footer_cwd(session_id, cwd)
//!   Footer poller (5s tick) → get_footer_cwd(session_id) → emit FooterStateUpdate
//!
//! Follows the same pattern as bash_abort.rs (BUG-129), tool_progress.rs (BUG-126),
//! tool_pause.rs (BUG-127), bridge_handler.rs (BUG-128).

use crate::session_registry::SessionRegistry;
use once_cell::sync::Lazy;
use uuid::Uuid;

/// Per-session last-known CWD for footer display.
/// Written by BashTool after resolve_cwd(), read by the footer poller each tick.
static LAST_KNOWN_CWD: Lazy<SessionRegistry<String>> = Lazy::new(SessionRegistry::new);

/// Update the last known CWD for a session.
/// Called by BashTool after resolving the effective working directory.
pub fn update_footer_cwd(session_id: Uuid, cwd: String) {
    LAST_KNOWN_CWD.set(session_id, Some(cwd));
}

/// Get the last known CWD for a session.
/// Called by the footer poller on each tick to detect CWD changes.
/// Returns None if no CWD has been recorded (session just created, no commands yet).
pub fn get_footer_cwd(session_id: Uuid) -> Option<String> {
    LAST_KNOWN_CWD.get(&session_id)
}

/// Remove the CWD entry for a session.
/// Called on session destroy to prevent memory leaks.
pub fn unregister_footer_cwd(session_id: Uuid) {
    LAST_KNOWN_CWD.remove(&session_id);
}
