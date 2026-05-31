//! Wire-shape conversions between the internal
//! `codelet_tools::tool_pause::*` pause type family and the
//! transport-portable `codelet_rpc_types::*` peers.
//!
//! Added by **RPC-042**. The two families exist for different reasons:
//!
//! * `codelet_tools::tool_pause` types are the *internal* contract used
//!   by the agent loop and the BackgroundSession pause handler — they
//!   carry a richer `PauseKind::Continue` variant (loop-control signal)
//!   and a `PauseResponse::{Resumed,Interrupted,...}` family that
//!   includes session-local control responses (`Interrupted`).
//! * `codelet_rpc_types` peers are the *wire-portable* slice consumed
//!   by the dual-transport `SessionManagerHandle` trait — they
//!   intentionally drop `Continue` (the doc-comment on
//!   `codelet_rpc_types::PauseKind` explicitly notes this) and replace
//!   the internal response taxonomy with the user-facing approval
//!   model (`ApprovalChoice`, `ConfirmAccept`, `ConfirmDeny`).
//!
//! This module is the one place both worlds meet so neither the impl
//! block nor the napi adapter has to repeat the mapping logic.
//!
//! ## Why free functions instead of `From` impls
//!
//! Rust's orphan rule forbids defining
//! `impl From<codelet_tools::tool_pause::PauseState> for codelet_rpc_types::PauseState`
//! inside `codelet-sessions` because **both** the source and target
//! types live in foreign crates. This module therefore exposes the
//! conversion as a free function `pause_state_to_rpc(...)` rather than
//! through `Into`/`From`.

use codelet_rpc_types::{
    ApprovalChoice, PauseKind as RpcPauseKind, PauseState as RpcPauseState,
};
use codelet_tools::tool_pause::{
    PauseKind as ToolPauseKind, PauseResponse as ToolPauseResponse,
    PauseState as ToolPauseState,
};

/// Map the internal `tool_pause::PauseState` shape onto the
/// wire-portable `codelet_rpc_types::PauseState`:
///
/// * `kind`: `Continue` and `Confirm` both collapse to
///   `RpcPauseKind::Confirm` (the wire enum intentionally omits
///   `Continue` per its doc-comment); `Triple` maps to
///   `RpcPauseKind::Triple`.
/// * `prompt`: concatenation of `tool_name` and `message` so the
///   single-string display surface on the wire shape carries both
///   pieces of context the dialog wants to render.
/// * `tool_call_id`: carried over verbatim from `details`.
pub fn pause_state_to_rpc(internal: ToolPauseState) -> RpcPauseState {
    let kind = match internal.kind {
        ToolPauseKind::Confirm | ToolPauseKind::Continue => RpcPauseKind::Confirm,
        ToolPauseKind::Triple => RpcPauseKind::Triple,
    };
    let prompt = if internal.tool_name.is_empty() {
        internal.message
    } else {
        format!("{}: {}", internal.tool_name, internal.message)
    };
    RpcPauseState {
        kind,
        prompt,
        tool_call_id: internal.details,
    }
}

/// Convert a user-surfaced `ApprovalChoice` (from the three-button
/// pause dialog) into the internal `tool_pause::PauseResponse` that
/// the BackgroundSession pause handler is waiting on.
///
/// Mapping (per the RPC-042 specification):
/// * `Approve` → `AllowOnce` (permit once, prompt again next time)
/// * `ApproveSession` → `AllowSession` (permit for the rest of the session)
/// * `Deny` → `Denied`
pub fn approval_choice_to_pause_response(choice: ApprovalChoice) -> ToolPauseResponse {
    match choice {
        ApprovalChoice::Approve => ToolPauseResponse::AllowOnce,
        ApprovalChoice::ApproveSession => ToolPauseResponse::AllowSession,
        ApprovalChoice::Deny => ToolPauseResponse::Denied,
    }
}

/// Convert a boolean accept/deny choice (from the two-button confirm
/// pause dialog) into the internal `tool_pause::PauseResponse`.
///
/// Mapping: `true` → `Approved`, `false` → `Denied`.
pub fn confirm_accept_to_pause_response(accept: bool) -> ToolPauseResponse {
    if accept {
        ToolPauseResponse::Approved
    } else {
        ToolPauseResponse::Denied
    }
}

// ============================================================================
// Internal ↔ wire conversions for `WorkUnitContext` and
// `CompactionProgress`.
//
// These two types have a *local* shape in `background_session.rs`
// (carrying `Option` fields with helper methods) and a *wire* shape
// in `codelet_rpc_types` (carrying required fields). The local types
// belong to `codelet-sessions`, so `From` impls across them are
// legal under the orphan rule.
// ============================================================================

impl From<crate::background_session::WorkUnitContext> for codelet_rpc_types::WorkUnitContext {
    fn from(internal: crate::background_session::WorkUnitContext) -> Self {
        codelet_rpc_types::WorkUnitContext {
            id: internal.id.unwrap_or_default(),
            title: internal.title.unwrap_or_default(),
            status: internal.status.unwrap_or_default(),
        }
    }
}

impl From<crate::background_session::CompactionProgress>
    for codelet_rpc_types::CompactionProgress
{
    fn from(internal: crate::background_session::CompactionProgress) -> Self {
        codelet_rpc_types::CompactionProgress {
            phase: internal.phase,
            current: internal.current,
            total: internal.total,
        }
    }
}
