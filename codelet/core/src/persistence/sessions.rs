//! Session deletion — thin alias preserved for backward compatibility.
//!
//! RPC-026 originally introduced this module as a transport-friendly
//! `delete_session(uuid)` helper that codelet-napi and codelet-rpc could
//! call into without re-introducing a `rpc → napi` arrow. RPC-033 lifted
//! the entire `SessionStore` into [`crate::persistence::manifest`], and
//! the canonical delete path is now
//! [`crate::persistence::manifest::delete_session`] (which clears the
//! lifted in-memory cache + the last-session bookkeeping + the on-disk
//! file in a single call).
//!
//! This module remains as a thin alias so the RPC-026 cross-transport
//! parity test in `codelet/fspec-tui/tests/rpc026_cross_transport_parity.rs`
//! continues to compile against the historical
//! `codelet_core::persistence::sessions::delete_session` path.

use uuid::Uuid;

/// Delete the on-disk session manifest for `id`.
///
/// Idempotent — silently succeeds when the file is already absent. Routes
/// through the lifted `manifest::delete_session` so the in-memory cache
/// stays consistent. Held over from RPC-026; equivalent to calling
/// `codelet_core::persistence::delete_session(id)` directly.
pub fn delete_session(id: Uuid) -> Result<(), String> {
    crate::persistence::manifest::delete_session(id)
}
