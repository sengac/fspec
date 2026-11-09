//! Shared NAPI-free tool-layer callbacks (WT-004, originally the napi-side
//! `init_block_notification_callbacks` wiring in `rust/napi/src/bridges.rs`).
//!
//! Before WT-004 the three tool-layer callbacks — the GIT-020 isolation
//! context lookup (`get_session_effective_cwd`), the BLOCK-006 work-unit
//! stage lookup (`get_session_work_unit_stage`), and the block-notification
//! emitter (`emit_block_notification_to_tui`) — lived ONLY in
//! `codelet-napi` and were registered into `codelet-tools`' process-global
//! `OnceLock`s exclusively from the NAPI adapter. The `fspec` binary
//! (`build_service`, combined/daemon mode) never registered them, so in the
//! binary:
//!
//! - `get_isolation_context(session_id)` → always `None`
//! - `validate_and_resolve_path(...)` → "no isolation context — allow all
//!   paths" (the documented fall-through)
//! - stage-gated writes (BLOCK-006) → never checked
//! - blocked actions → no `UserNotification` chunk
//!
//! i.e. an isolated session in the Rust TUI could read/write the main
//! project root, `/tmp`, anything — the worktree was a cosmetic badge, not
//! a boundary.
//!
//! This module is the single source of truth: the `fspec` binary's
//! `build_service` registers its NON-singleton `SessionManager` via
//! [`register_manager`] and wires the three shared functions into
//! `codelet-tools`. (The NAPI front door that relied on a
//! `SessionManager::instance()` fallback is gone; BUG-398 removed that
//! fallback from the session lookup.)
//!
//! The manager slot mirrors the [`crate::footer_poller`] chunk-sender slot
//! pattern (WT-002): a process-global `Mutex<Option<Arc<SessionManager>>>`
//! that the fspec binary registers.

#![allow(clippy::expect_used)]

use std::sync::{Arc, Mutex as StdMutex};

use codelet_rpc_types::{NotificationSeverity, SessionId, StreamChunk};
use codelet_tools::facade::IsolationContext;

use crate::background_session::BackgroundSession;
use crate::session_manager::SessionManager;

/// Process-global tool-callbacks manager (WT-004).
///
/// The `fspec` binary owns a NON-singleton `SessionManager` (held as an
/// `Arc` by `build_service`) whose sessions are the only ones its TUI
/// drives; registering it here makes the shared callbacks consult THAT
/// manager, and only that manager ([`session_by_id`]).
static TOOL_CALLBACKS_MANAGER: StdMutex<Option<Arc<SessionManager>>> = StdMutex::new(None);

/// Register the `SessionManager` that the shared tool callbacks look up
/// sessions in. Replaces any previous registration.
///
/// Must be called BEFORE any session is created (the fspec binary's
/// `build_service` calls it during service construction).
pub fn register_manager(manager: Arc<SessionManager>) {
    *TOOL_CALLBACKS_MANAGER
        .lock()
        .expect("tool callbacks manager lock poisoned") = Some(manager);
}

/// Test-only: clear the manager registration (no session is found until a
/// manager is registered again).
pub fn unregister_manager() {
    *TOOL_CALLBACKS_MANAGER
        .lock()
        .expect("tool callbacks manager lock poisoned") = None;
}

/// Look up a session by id in the registered manager, and only there
/// (BUG-398: the NAPI-era `SessionManager::instance()` fallback is gone
/// with the NAPI front door).
///
/// BUG-404: an unparseable id, a session the registered manager does not
/// know and a missing manager registration are errors naming the session —
/// the isolation lookup must not read them as "not isolated".
fn session_by_id(session_id_str: &str) -> Result<Arc<BackgroundSession>, String> {
    let id = uuid::Uuid::parse_str(session_id_str)
        .map_err(|error| format!("invalid session id {session_id_str}: {error}"))?;
    let session_id = codelet_tools::tool_admission::authorization_identity(id).to_string();
    let registered = TOOL_CALLBACKS_MANAGER
        .lock()
        .expect("tool callbacks manager lock poisoned")
        .clone();
    match registered {
        Some(manager) => manager.get_session(&session_id),
        None => Err(format!(
            "no tool-callbacks manager is registered (register_manager was not called); \
             session {session_id} cannot be looked up"
        )),
    }
}

/// GIT-020: resolve the isolation context for a session (shared callback).
///
/// For isolated sessions, returns `Ok(Some(IsolationContext))` with:
/// - `worktree_path`: where file operations ARE allowed (the worktree)
/// - `blocked_project_path`: where file operations are BLOCKED (the session
///   project root)
///
/// A found non-isolated session returns `Ok(None)` (unrestricted). BUG-404:
/// a failed lookup returns `Err`, and the tool layer refuses the operation.
///
/// Signature matches `codelet_tools::facade::GetEffectiveCwdCallback`.
pub fn isolation_context(session_id_str: String) -> Result<Option<IsolationContext>, String> {
    // An auxiliary agent with no owning session (e.g. compaction outside a
    // tool invocation) has no session or worktree behind it.
    if uuid::Uuid::parse_str(&session_id_str)
        .is_ok_and(codelet_tools::tool_admission::is_standalone_auxiliary)
    {
        return Ok(None);
    }
    let session = session_by_id(&session_id_str)?;
    // Only isolated sessions carry a worktree; everything else stays
    // unrestricted.
    Ok(session
        .worktree_path()
        .map(|worktree_path| IsolationContext {
            worktree_path,
            blocked_project_path: std::path::PathBuf::from(&session.project),
        }))
}

/// BLOCK-006: resolve the current work unit stage for a session (shared
/// callback).
///
/// Returns the work unit context's `status` (e.g. "testing",
/// "implementing") and the root its files live under (the worktree of an
/// isolated session, else the project) when the session has a work unit
/// context with a status; `Ok(None)` when it has none (no stage gating).
/// BUG-449: a failed lookup is an error and the write is refused.
///
/// Signature matches `codelet_tools::facade::GetWorkUnitStageCallback`.
pub fn work_unit_stage(
    session_id_str: String,
) -> Result<Option<codelet_tools::stage_gate::SessionStage>, String> {
    let session = session_by_id(&session_id_str)?;
    Ok(session
        .get_work_unit_context()
        .and_then(|ctx| ctx.status)
        .map(|stage| codelet_tools::stage_gate::SessionStage {
            stage,
            project_root: session
                .worktree_path()
                .unwrap_or_else(|| std::path::PathBuf::from(&session.project)),
        }))
}

/// BLOCK-006: emit a block notification to the session's chunk stream
/// (shared callback).
///
/// Sends a `UserNotification` chunk with `Warning` severity, formatted
/// `"AI was blocked from {action} - {reason}"`, on the registered
/// manager-owned `chunks_tx` (the [`crate::footer_poller`] chunk-sender
/// slot, singleton fallback). Both front doors register the correct
/// sender: the fspec binary's `build_service` registers its non-singleton
/// manager's `chunks_tx`; the NAPI path re-registers the singleton's on
/// every footer-poller spawn.
///
/// Signature matches `codelet_tools::facade::BlockNotificationCallback`.
pub fn emit_block_notification(session_id_str: String, action: String, reason: String) {
    let message = format!("AI was blocked from {action} - {reason}");
    let chunk = StreamChunk::user_notification(message, NotificationSeverity::Warning);
    // BUG-405: no registered sender and no live receiver are both reported
    // (the block itself already happened; only the notice is undeliverable).
    match crate::footer_poller::emission_target() {
        Ok(sender) => {
            if let Err(error) = sender.send((SessionId::from(session_id_str.clone()), chunk)) {
                tracing::error!(session = %session_id_str, %error, "block notification not delivered: no receiver");
            }
        }
        Err(error) => {
            tracing::error!(session = %session_id_str, %error, "block notification not delivered");
        }
    }
}
