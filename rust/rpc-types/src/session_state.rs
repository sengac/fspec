//! Per-session derived state, pause/HITL and isolation wire types (RPC-036).

use super::*;

// ============================================================================
// RPC-036: per-session derived state + pause/HITL wire types + isolation
// supporting types. Added so the Rust AgentView and the TypeScript Ink
// frontend can share identical wire shapes via the `napi` feature gate.
// ============================================================================

// ---------------------------------------------------------------------------
// Phase 2.1 — Per-session derived state
// ---------------------------------------------------------------------------

/// Per-session token totals (input + output) as currently observed.
/// Mirrors what `BackgroundSession::get_tokens()` produces on the JS
/// side.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTokens {
    pub input_tokens: i64,
    pub output_tokens: i64,
}

/// Per-session token-restore state — used by `/resume` to rehydrate the
/// cumulative-billed counters and cache totals so the SessionFooter's
/// context-fill / billing badges keep their numbers across reopens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TokenRestoreState {
    pub current_context: i64,
    pub cumulative_billed_output: i64,
    pub cache_read: i64,
    pub cache_creation: i64,
    pub cumulative_billed_input: i64,
    pub cumulative_billed_output_second: i64,
}

/// Per-session model binding: which provider+model the session uses
/// plus the derived limits (context window, max output tokens, and the
/// compaction threshold at which the session manager auto-compacts).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionModel {
    pub provider_id: String,
    pub model_id: String,
    /// PROV-166: `None` when the limit is unavailable.
    pub context_window: Option<i64>,
    /// PROV-166: `None` when the limit is unavailable.
    pub max_output_tokens: Option<i64>,
    /// PROV-166: `None` unless both limits are available.
    pub compaction_threshold: Option<i64>,
}

/// The work-unit this session is currently attached to (BoardView →
/// AgentView attach path). `None` when the session is detached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkUnitContext {
    pub id: String,
    pub title: String,
    pub status: String,
}

/// Provider-specific thinking-config payload. The `config_json` field
/// is the JSON-encoded blob produced by `getThinkingConfig(providerId,
/// level)` on the JS side — store it as a string instead of
/// `serde_json::Value` to keep `codelet-rpc-types` free of any
/// `serde_json` runtime dependency. Mirrors the precedent set by
/// `FspecRequest::args_json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ThinkingConfig {
    pub provider_id: String,
    pub level: ThinkingLevel,
    pub config_json: String,
}

// ---------------------------------------------------------------------------
// Phase 2.2 — Pause & HITL wire types
// ---------------------------------------------------------------------------

/// Kind of pause the user-facing dialog should render. Wire mirror of the
/// internal `codelet_tools::tool_pause::PauseKind`. AMGR-023: a human
/// action (the internal `Continue`: WebSearch 'interact with the page',
/// /goal 'human review needed') is `HumanAction`, appended so the earlier
/// variants keep their wire indices (RPC-449); it is answered with
/// `pause_human_action` only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PauseKind {
    Confirm,
    Triple,
    HumanAction,
}

/// AMGR-023: the user's reply to a human action. `Done` lets the waiting
/// tool go on; `Cancel` gives it the result an interrupt gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum HumanActionReply {
    Done,
    Cancel,
}

/// Snapshot of the AgentView pause dialog state — the question to
/// render plus the tool-call ID (if any) the pause is gating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PauseState {
    pub kind: PauseKind,
    pub prompt: String,
    /// BUG-291: the pause details text (formerly misnamed `tool_call_id`;
    /// same bincode position and type).
    pub details: Option<String>,
    /// BUG-291: the id of the tool call that raised the pause.
    pub tool_call_id: Option<String>,
}

/// Response the user can send back to dismiss the pause dialog. Maps
/// 1-to-1 onto the AgentView buttons (`Resume`, `ConfirmAccept`,
/// `ConfirmDeny` for two-choice prompts; `TripleApprove`,
/// `TripleApproveSession`, `TripleDeny` for three-choice prompts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PauseResponse {
    Resume,
    ConfirmAccept,
    ConfirmDeny,
    TripleApprove,
    TripleApproveSession,
    TripleDeny,
}

/// Per-blocklist approval choice surfaced by the triple-pause dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum ApprovalChoice {
    Approve,
    ApproveSession,
    Deny,
}

/// One option presented to the user inside an HITL question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitlOption {
    pub label: String,
    pub description: String,
}

/// Single HITL question — wire-facing mirror of
/// `codelet_tools::request_user_input::HitlQuestion` (RPC-410). An
/// empty `options` vec means a pure freeform question; a question WITH
/// options still accepts freeform text via the virtual "Other..."
/// entry — freeform capability is derived exactly as the TypeScript
/// frontend derives it (there is no `allow_text_input` flag).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitlQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    /// BUG-403: required on the wire; send `[]` for a freeform question.
    pub options: Vec<HitlOption>,
}

/// Active HITL request — carries the FULL array of 1–3 questions in
/// order (RPC-410: TS parity with `HitlRequestInfo { questions }`; no
/// first-question slicing).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitlRequest {
    pub questions: Vec<HitlQuestion>,
}

/// One structured answer for one HITL question (RPC-410: TS parity
/// with `HitlAnswer { id, selected, other }`). Option answers carry
/// the chosen labels in `selected` with `other: None`; freeform /
/// "Other..." answers carry `selected: []` and the typed text in
/// `other`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct HitlAnswer {
    pub id: String,
    pub selected: Vec<String>,
    pub other: Option<String>,
}

/// User's response to an `HitlRequest` (RPC-410: TS parity with
/// `{ cancelled, answers }`). Cancel (Esc) is `cancelled: true` with
/// an empty `answers` vec; submit is `cancelled: false` with one
/// answer per question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct HitlResponse {
    pub cancelled: bool,
    /// BUG-403: required on the wire; a cancel sends `[]`.
    pub answers: Vec<HitlAnswer>,
}

/// TOOL-022 P2: deterministic exec-stdin prompt request — surfaced in
/// the TUI composer slot when a live unified_exec session has been
/// quiet for >= 3s while its child is alive. Wire-facing mirror of
/// the tools-internal `codelet_tools::unified_exec::ExecStdinRequest`
/// (NO hint/content field — nothing derived from output content
/// crosses the wire). `quiet_seconds` / `ts_ms` are `i64` (not `u64`)
/// because the `napi(object)` derive in this workspace only supports
/// i32/i64/f32/f64 numerics — matching the `LogRecord.timestamp_ms`
/// convention; the values are always non-negative in practice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecStdinRequest {
    /// The unified_exec session id (NOT the agent session id).
    pub exec_session_id: String,
    /// Command display string.
    pub command: String,
    /// Seconds since last output when the detector fired (floored).
    pub quiet_seconds: i64,
    /// Detector fire time, Unix epoch milliseconds.
    pub ts_ms: i64,
}

// ---------------------------------------------------------------------------
// Phase 2.4 — Supporting types
// ---------------------------------------------------------------------------

/// Result of `create_isolated_session`: identifies the new session,
/// its git worktree path on disk, and the baseline commit SHA the
/// worktree was forked from. Drives the AgentView isolation badge and
/// the merge/discard flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolatedSessionInfo {
    pub session_id: SessionId,
    pub worktree_path: String,
    pub base_commit: String,
}
