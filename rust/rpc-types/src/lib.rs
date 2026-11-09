//! codelet-rpc-types: shared serde types for the fspec dual-transport RPC.
//!
//! Single source of truth for any type that crosses the RPC boundary in
//! either direction. Default builds have zero dependencies on tarpc, tokio,
//! or napi — those crates depend on us, never the other way around.
//!
//! ## NAPI feature gate
//!
//! Enabling the `napi` feature applies `#[napi(object)]` (or
//! `#[napi(discriminant = "type")]` / `#[napi(string_enum)]`) to types that
//! cross the JS boundary so that `rust/napi` can re-export them
//! verbatim and preserve the existing TypeScript shape (most notably
//! `correlationId`/`observedCorrelationIds`/`toolCall` etc. via
//! `#[napi(js_name = ...)]`). The feature is off by default; only
//! `codelet-napi` opts in.
//!
//! RPC-005 lifted only `WorkUnitInfo` from `rust/napi/src/types.rs:182`.
//! RPC-007 lifts five additional types that the session REPL needs as a
//! single source of truth shared by the embedded transport, the WebSocket
//! transport, and the NAPI surface:
//!   * [`SessionId`] — newtype around String
//!   * [`SessionInfo`] — opaque metadata returned by `list_sessions`
//!   * [`SessionStatus`] — coarse session lifecycle state
//!   * [`StreamChunk`] — the 23-variant streaming chunk discriminated union
//!     plus its supporting structs
//!   * [`LogRecord`] — structured tracing event payload

use serde::{Deserialize, Serialize};

mod transcript;
mod transcript_assembler;
pub use transcript::{
    TranscriptCursor, TranscriptError, TranscriptFragment, TranscriptOriginal, TranscriptPage,
};
pub use transcript_assembler::TranscriptAssembler;

mod committed_compaction;
mod provider_settings;
pub use committed_compaction::CommittedContextSnapshot;
pub use provider_settings::{ProviderProfileConfig, ProviderSettingsConfig};
mod request_context;
pub use request_context::{ContextProvenance, RequestContextSnapshot};
mod work_unit_focus;
pub use work_unit_focus::WorkUnitFocus;
mod exec_stdin_control;
pub use exec_stdin_control::ExecStdinControl;
mod input_mode;
mod session_mcp;
pub use input_mode::InputMode;
pub use session_mcp::{SessionMcpServer, SessionMcpServerTools, SessionMcpTransport};
mod session_manifest_issue;
pub use session_manifest_issue::SessionManifestIssue;

// Per-surface wire-type modules (REFAC-021); every item is re-exported at
// the crate root so `codelet_rpc_types::<Type>` paths are unchanged.
mod work_units;
pub use work_units::*;
mod git_state;
pub use git_state::*;
mod sessions;
pub use sessions::*;
mod chrome;
pub use chrome::*;
mod thinking;
pub use thinking::*;
mod dialogs;
pub use dialogs::*;
mod credentials;
pub use credentials::*;
mod operations;
pub use operations::*;
mod session_card_changes;
pub use session_card_changes::{
    CardMoveBack, CardMoveBackOutcome, SessionCardChange, SessionCardChangeKind,
};
mod chunk_support;
pub use chunk_support::*;
mod session_state;
pub use session_state::*;
// AMGR-020: questions and approvals are requests with an id, origin and chain.
mod interaction_request;
pub use interaction_request::*;
mod stream_chunk;
pub use stream_chunk::*;
// BUG-648: which chunks a session journals.
mod chunk_journal;
mod stream_chunk_constructors;

#[cfg(test)]
mod tests;
