//! Session deletion lifted into codelet_core (RPC-026).
//!
//! Provides a transport-friendly `delete_session(uuid)` helper that
//! both the codelet-napi NAPI surface AND the codelet-rpc
//! `FspecService` impl can call into without re-introducing a
//! `rpc → napi` dependency.
//!
//! Storage layout matches the NAPI `SessionStore` byte-for-byte:
//! each session is persisted as
//! `{codelet_common::get_data_dir()}/sessions/{uuid}.json`. Deletion
//! removes that file (idempotent — missing files are treated as
//! already-deleted).
//!
//! The NAPI `SessionStore` retains an in-memory cache; that cache is
//! invalidated lazily via `SessionStore::new()` on next call (the
//! pre-RPC-026 behaviour). This module deliberately does NOT touch the
//! NAPI cache because codelet_core has no dependency on codelet_napi.
//! Callers that hold a long-lived `SessionStore` must rebuild it after
//! invoking this helper.
//!
//! Returns `Err(String)` only on I/O failure. Missing files are
//! mapped to `Ok(())` so the function is idempotent (matches the
//! pre-RPC-026 NAPI behaviour where deleting an unknown id silently
//! succeeded because the in-memory cache reported no hit).
//!
//! Keep this file under 100 LoC.

use std::fs;
use uuid::Uuid;

/// Delete the on-disk session manifest for `id`.
///
/// Idempotent — silently succeeds when the file is already absent.
pub fn delete_session(id: Uuid) -> Result<(), String> {
    let sessions_dir = codelet_common::get_data_dir()?.join("sessions");
    let path = sessions_dir.join(format!("{id}.json"));
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(&path)
        .map_err(|e| format!("Failed to delete session file {}: {e}", path.display()))
}
