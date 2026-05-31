# RPC-036 — Widen `codelet-rpc-types` with every wire-portable shape AgentView needs

**Parent:** RPC-030 · **Phase:** 2.1-2.4 · **Estimate:** 8 pts · **Depends on:** RPC-035

## Goal

Add every data shape that crosses the AgentView ↔ session-manager boundary on the TypeScript side as a peer in `codelet-rpc-types`. After this card, nothing AgentView reads/writes needs to touch `napi::` types — Rust frontend and TS frontend both use these shared shapes.

## Current state of `codelet/rpc-types/src/lib.rs`

847 lines. Already defined (from RPC-007 / RPC-025 / RPC-026):

- `WorkUnitInfo`, `CheckpointCounts`, `SessionId`, `SessionInfo`, `LogRecord`, `HealthInfo`, `ModelInfo`, `WorkspaceInfo`, `ModelEntry`, `ProviderInfo`, `CompactionProgress`, `ToolCallInfo`, `ToolResultInfo`, `ToolProgressInfo`, `ContextFillInfo`, `SupervisorPendingInjectionInfo`, `IncomingMessageImage`, `CompactionResult`, `HistoryMatch`, `FspecRequest`, `FspecResult`, `TokenTracker`
- Enums: `SessionStatus`, `SessionState`, `NotificationSeverity`, `ThinkingLevel`
- `StreamChunk` with **22 variants** including `SessionStateChange`, `IsolationStateChange`, `DebugStateChange`, `FooterStateUpdate`, `FspecCommandRequest`, `FspecCommandResult`, `CompactionComplete`, `SupervisorPendingInjection`, `Interrupted`

## What to add

All new types get `#[derive(Debug, Clone, Serialize, Deserialize)]` plus `#[cfg_attr(feature = "napi", napi_derive::napi(object))]`.

### Step 2.1 — Per-session derived state

```rust
pub struct SessionTokens { pub input_tokens: i64, pub output_tokens: i64 }

pub struct TokenRestoreState {
    pub current_context: i64,
    pub cumulative_billed_output: i64,
    pub cache_read: i64,
    pub cache_creation: i64,
    pub cumulative_billed_input: i64,
    pub cumulative_billed_output_second: i64,
}

pub struct SessionModel {
    pub provider_id: String,
    pub model_id: String,
    pub context_window: i64,
    pub max_output_tokens: i64,
    pub compaction_threshold: i64,
}

pub struct WorkUnitContext { pub id: String, pub title: String, pub status: String }

pub struct ThinkingConfig {
    // Provider-specific JSON-shaped struct.
    // Mirror what `getThinkingConfig(providerId, level)` produces in
    // src/llm/thinking-config.ts. Use `serde_json::Value` if shape varies per provider.
    pub provider_id: String,
    pub level: ThinkingLevel,
    pub config: serde_json::Value,
}
```

### Step 2.2 — Pause & HITL

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PauseKind { Confirm, Triple }

pub struct PauseState {
    pub kind: PauseKind,
    pub prompt: String,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PauseResponse {
    Resume,
    ConfirmAccept,
    ConfirmDeny,
    TripleApprove,
    TripleApproveSession,
    TripleDeny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApprovalChoice { Approve, ApproveSession, Deny }

pub struct HitlOption {
    pub label: String,
    pub description: String,
}

pub struct HitlRequest {
    pub id: String,
    pub question: String,
    pub header: String,
    pub options: Vec<HitlOption>,
    pub allow_text_input: bool,
}

pub struct HitlResponse { pub id: String, pub value: String }
```

Mirror `codelet_tools::tool_pause::{PauseKind, PauseRequest, PauseResponse, PauseState}` and `codelet_tools::request_user_input::{HitlRequest, HitlResponse}` — those are the source types currently used internally by `BackgroundSession`.

### Step 2.3 — `StreamChunk` audit

**Already present in `StreamChunk` (no work needed):**
- `SessionStateChange { state: SessionState }` ✓
- `IsolationStateChange { is_isolated, worktree_path }` ✓
- `DebugStateChange { enabled }` ✓
- `FooterStateUpdate { cwd, display_path, is_git_repo, branch }` ✓
- `FspecCommandRequest { fspec_request: FspecRequest }` ✓
- `FspecCommandResult { fspec_result: FspecResult }` ✓
- `CompactionComplete { compaction_result: CompactionResult }` ✓
- `SupervisorPendingInjection { ... }` ✓
- `Interrupted { queued_inputs }` ✓

**To add (if missing parity with TS):**

Audit `src/tui/utils/streamChunks.ts` (or equivalent) for any chunk type the TS frontend emits that lacks a Rust peer. Known additions from the roadmap:

- `IsolationStateChange` — already there but the original roadmap added `base_commit: String` — confirm and add if needed.

### Step 2.4 — Supporting types

```rust
pub struct IsolatedSessionInfo {
    pub session_id: SessionId,
    pub worktree_path: String,
    pub base_commit: String,
}
```

`FspecResult` (already present) requires audit: ensure fields match `codelet/napi/src/types.rs::FspecResult` for byte-compatible round-trip through NAPI.

## Audit checklist

| Type | TS source | Rust source today |
|---|---|---|
| `SessionTokens` | `src/llm/state.ts` token usage | `BackgroundSession::get_tokens()` (line 877 of session_manager.rs) |
| `TokenRestoreState` | `src/llm/token-restore.ts` | `session_restore_token_state` (line 7692) |
| `SessionModel` | `src/llm/model-config.ts` | `session_get_model` (line 7144) |
| `ThinkingConfig` | `src/llm/thinking-config.ts` | resolved in `BackgroundSession::set_base_thinking_level` (line 1156) |
| `WorkUnitContext` | `src/tui/store/sessionStore.ts` | `BackgroundSession::work_unit_context` (line 567) |
| `PauseState` etc. | `src/llm/tools/pause.ts` | `codelet_tools::tool_pause` |
| `HitlRequest` etc. | `src/llm/tools/request-user-input.ts` | `codelet_tools::request_user_input` |
| `IsolatedSessionInfo` | `src/tui/services/isolated-session.ts` | `IsolatedSessionResult` in session_manager.rs (line 3542) |

## NAPI feature behaviour

With `feature = "napi"`, `napi_derive::napi(object)` generates a TS interface. Confirm `codelet/napi/index.d.ts` after `cargo build --features napi` contains the new interfaces in their expected camelCase form.

## Acceptance criteria

1. All structs/enums listed above exist in `codelet/rpc-types/src/lib.rs` with `Serialize + Deserialize` and (where applicable) `#[cfg_attr(feature = "napi", napi_derive::napi(object))]`.
2. `cargo build -p codelet-rpc-types` passes (default features).
3. `cargo build -p codelet-rpc-types --features napi` passes.
4. `cargo build -p codelet-napi` passes (the napi crate consumes these via re-export).
5. JSON round-trip tests for each new type (serialize → deserialize → assert equal).
6. `codelet/napi/index.d.ts` regenerated — diff shows ONLY additions, no removals or renames.

## Out of scope

- Wiring these types through `SessionManagerHandle` and `FspecService` → RPC-037.
- Wiring the AgentView → RPC-045 onwards.
