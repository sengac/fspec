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
//! This module is the single source of truth (two-front-doors rule):
//!
//! - the `fspec` binary's `build_service` registers its NON-singleton
//!   `SessionManager` via [`register_manager`] and wires the three shared
//!   functions into `codelet-tools`;
//! - `codelet-napi`'s `init_block_notification_callbacks` registers the
//!   SAME shared functions — without calling `register_manager`, so the
//!   manager slot stays empty and lookups fall back to
//!   [`SessionManager::instance()`], which is exactly the session store the
//!   NAPI adapter drives (today's behavior, byte-for-byte).
//!
//! The manager slot mirrors the [`crate::footer_poller`] chunk-sender slot
//! pattern (WT-002): a process-global `Mutex<Option<Arc<SessionManager>>>`
//! that the fspec binary registers, with a singleton fallback for NAPI.

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
/// manager. The NAPI path never registers — its sessions live in
/// `SessionManager::instance()`, which [`session_by_id`] falls back to.
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

/// Test-only: clear the manager registration so the singleton fallback is
/// in effect again (mirrors a fresh NAPI process).
pub fn unregister_manager() {
    *TOOL_CALLBACKS_MANAGER
        .lock()
        .expect("tool callbacks manager lock poisoned") = None;
}

/// Look up a session by id, consulting the registered manager when present
/// and falling back to the singleton otherwise.
///
/// Returns `None` for unparseable ids and for sessions the relevant manager
/// does not know about — both degrade to "no isolation / no stage" at the
/// tool layer (today's allow-all behavior, never stricter).
fn session_by_id(session_id_str: &str) -> Option<Arc<BackgroundSession>> {
    let registered = TOOL_CALLBACKS_MANAGER
        .lock()
        .expect("tool callbacks manager lock poisoned")
        .clone();
    match registered {
        Some(manager) => manager.get_session(session_id_str).ok(),
        None => SessionManager::instance().get_session(session_id_str).ok(),
    }
}

/// GIT-020: resolve the isolation context for a session (shared callback).
///
/// For isolated sessions, returns `Some(IsolationContext)` with:
/// - `worktree_path`: where file operations ARE allowed (the worktree)
/// - `blocked_project_path`: where file operations are BLOCKED (the session
///   project root)
///
/// For non-isolated sessions, unknown session ids, and unparseable ids,
/// returns `None` — the tool layer then skips path validation entirely
/// (today's NAPI behavior, unchanged).
///
/// Signature matches `codelet_tools::facade::GetEffectiveCwdCallback`
/// exactly (plain function pointer — no OnceLock type changes in
/// codelet-tools, business rule 2).
pub fn isolation_context(session_id_str: String) -> Option<IsolationContext> {
    let session = session_by_id(&session_id_str)?;
    // Only isolated sessions carry a worktree; everything else stays
    // unrestricted.
    let worktree_path = session.worktree_path()?;
    Some(IsolationContext {
        worktree_path,
        blocked_project_path: std::path::PathBuf::from(&session.project),
    })
}

/// BLOCK-006: resolve the current work unit stage for a session (shared
/// callback).
///
/// Returns the work unit context's `status` (e.g. "testing",
/// "implementing") when the session has a work unit context set, else
/// `None` (no stage-gating for that session — `check_write_permission`
/// treats a missing stage as "allow all").
///
/// Signature matches `codelet_tools::facade::GetWorkUnitStageCallback`.
pub fn work_unit_stage(session_id_str: String) -> Option<String> {
    session_by_id(&session_id_str)?
        .get_work_unit_context()
        .and_then(|ctx| ctx.status)
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
    let sender = crate::footer_poller::emission_target();
    let _ = sender.send((SessionId::from(session_id_str), chunk));
}
