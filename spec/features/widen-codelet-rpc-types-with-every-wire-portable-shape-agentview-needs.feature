@done
@schema-design
@session-management
@napi
@rust
@rpc
@RPC-036
Feature: Widen codelet-rpc-types with every wire-portable shape AgentView needs
  """
  Field-naming/napi-rename rule: types whose Rust field names already match the desired TS camelCase (e.g. SessionTokens.input_tokens → inputTokens) rely on napi-derive's automatic snake-to-camel conversion — no explicit `#[napi(js_name = ...)]` is needed. Fields with non-trivial renames (none in this card) would use explicit js_name. This matches the precedent set by WorkUnitInfo, SessionInfo, and TokenTracker in the same file.
  ThinkingConfig.config_json field rationale: codelet-rpc-types currently has zero serde_json dependency (only serde + optional napi). Using `serde_json::Value` in a `#[napi(object)]` struct is not natively supported by napi-derive v3 and would require either an extra dependency on serde_json plus a custom NAPI-to-JsObject conversion shim, or `#[napi(ts_type = "any")]` hacks. Following the existing FspecRequest::args_json precedent, we store the provider-specific config as a JSON-encoded String — callers parse via `serde_json::from_str` on the consumer side (codelet-sessions / codelet-fspec-tui already have serde_json as a dependency). This keeps codelet-rpc-types dependency-light and the TS side gets a simple `configJson: string` field that JSON.parses cleanly.
  PauseKind/PauseResponse/ApprovalChoice wire shape vs internal codelet_tools shape: the existing codelet_tools::tool_pause module owns PauseKind { Continue, Confirm, Triple } and PauseResponse { Resumed, Approved, Denied, Interrupted, AllowOnce, AllowSession } for the session-internal pause loop. The wire-portable shapes added here are deliberately distinct — they model only the AgentView-facing states (pause kinds the user can choose between in the UI and the responses the user can send back). Phase 4 (extract codelet-sessions) will map between the internal codelet_tools types and the wire types at the SessionManager boundary. Keeping the two type families separate avoids polluting codelet-rpc-types with internal session-loop concerns (Continue/Resumed/Interrupted are loop control signals, not user-facing choices).
  IsolationStateChange backward-compat: the existing StreamChunk::IsolationStateChange variant has { is_isolated, worktree_path: Option<String> } today (see codelet/rpc-types/src/lib.rs:622-627). Adding `base_commit: Option<String>` as a new field is forward-compatible for serde JSON deserialization (Option fields default to None when missing) but the StreamChunk::isolation_state_change constructor at lib.rs:821 must be updated to accept and pass through the new field. Existing call sites that pass two args will need to be updated — search via `Grep('StreamChunk::isolation_state_change|isolation_state_change(', '*.rs')` before merging to ensure no caller is left compiling against the 2-arg signature.
  Test placement: round-trip tests live in codelet/rpc-types/src/lib.rs under a `#[cfg(test)] mod tests { use super::*; use serde_json; ... }` block at the bottom of the file. `serde_json` is added to `[dev-dependencies]` in codelet/rpc-types/Cargo.toml (not [dependencies]) to keep the default release build free of serde_json. This matches the codelet-core/codelet-common dev-dep pattern.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every new struct/enum lives in codelet/rpc-types/src/lib.rs with #[derive(Debug, Clone, Serialize, Deserialize)] and (where it crosses the JS boundary) #[cfg_attr(feature = "napi", napi_derive::napi(object))] for structs or #[cfg_attr(feature = "napi", napi_derive::napi(string_enum))] for enums — matching the established pattern set by WorkUnitInfo, SessionStatus, and ThinkingLevel in the same file
  #   2. Phase 2.1 per-session derived-state types are added: SessionTokens { input_tokens, output_tokens }, TokenRestoreState { current_context, cumulative_billed_output, cache_read, cache_creation, cumulative_billed_input, cumulative_billed_output_second }, SessionModel { provider_id, model_id, context_window, max_output_tokens, compaction_threshold }, and WorkUnitContext { id, title, status } — all using i64/u32 (not u64) so napi(object) compiles under napi-derive v3 + napi4 feature
  #   3. ThinkingConfig is added with provider_id: String, level: ThinkingLevel, and config_json: String (a JSON-encoded provider-specific blob) — using a string-encoded JSON field instead of serde_json::Value to keep the existing serde_json-free dependency footprint of codelet-rpc-types and to match the established FspecRequest::args_json pattern already in the file
  #   4. Phase 2.2 pause and HITL wire types are added: PauseKind enum (Confirm, Triple), PauseState { kind, prompt, tool_call_id: Option<String> }, PauseResponse enum (Resume, ConfirmAccept, ConfirmDeny, TripleApprove, TripleApproveSession, TripleDeny), ApprovalChoice enum (Approve, ApproveSession, Deny), HitlOption { label, description }, HitlRequest { id, question, header, options, allow_text_input }, and HitlResponse { id, value } — mirroring the wire-facing shape the AgentView reads and writes (not the internal codelet_tools::tool_pause / codelet_tools::request_user_input variants that include extra session-loop concerns)
  #   5. Phase 2.3 StreamChunk audit: the existing 22 variants (SessionStateChange, IsolationStateChange, DebugStateChange, FooterStateUpdate, FspecCommandRequest, FspecCommandResult, CompactionComplete, SupervisorPendingInjection, Interrupted, and the 13 other variants) are preserved unchanged. The only modification is that the IsolationStateChange variant gains an additional field `base_commit: Option<String>` (camelCase `baseCommit` on the napi side) so the Rust AgentView can render isolation diff baselines against the worktree origin commit — additive, no removals, no renames
  #   6. Phase 2.4 supporting type: IsolatedSessionInfo { session_id: SessionId, worktree_path: String, base_commit: String } is added. The pre-existing FspecResult type is left byte-compatible: its current shape (success, data: String, error: Option<String>, system_reminder, tool_call_id) is preserved so codelet/napi/src/types.rs::FspecResult round-trips through NAPI unchanged
  #   7. Every added struct and enum is exercised by a JSON round-trip test (serialize via serde_json → deserialize → assert equality with the original instance) in codelet/rpc-types/src/lib.rs under a #[cfg(test)] mod tests block — establishing the first test suite for rpc-types and proving the types are wire-portable by construction
  #   8. cargo build -p codelet-rpc-types (default features) and cargo build -p codelet-rpc-types --features napi BOTH succeed; cargo build -p codelet-napi --features napi continues to succeed (consumes the new types via the existing pub use chain); cargo test -p codelet-rpc-types runs the new round-trip suite green
  #   9. codelet/napi/index.d.ts regenerated after this card shows ONLY additions (new TypeScript interfaces for SessionTokens, TokenRestoreState, SessionModel, WorkUnitContext, ThinkingConfig, PauseKind, PauseState, PauseResponse, ApprovalChoice, HitlOption, HitlRequest, HitlResponse, IsolatedSessionInfo, plus a new optional `baseCommit?: string` field on the existing IsolationStateChange variant) — no existing TS interface is renamed, removed, or has any field reordered or removed
  #
  # EXAMPLES:
  #   1. An engineer running `cargo build -p codelet-rpc-types` (no features) succeeds without warnings — the new structs and enums compile under the default zero-dependency-on-napi build, proving they remain a pure-serde contract
  #   2. An engineer running `cargo build -p codelet-rpc-types --features napi` succeeds and `cargo build -p codelet-napi --release` (which enables the napi feature transitively) regenerates codelet/napi/index.d.ts to include exactly these new TS interfaces in camelCase: `interface SessionTokens { inputTokens: number; outputTokens: number }`, `interface TokenRestoreState { currentContext: number; cumulativeBilledOutput: number; cacheRead: number; cacheCreation: number; cumulativeBilledInput: number; cumulativeBilledOutputSecond: number }`, `interface SessionModel { providerId: string; modelId: string; contextWindow: number; maxOutputTokens: number; compactionThreshold: number }`, `interface WorkUnitContext { id: string; title: string; status: string }`, `interface ThinkingConfig { providerId: string; level: ThinkingLevel; configJson: string }`, `interface PauseState { kind: PauseKind; prompt: string; toolCallId?: string | undefined | null }`, `interface HitlOption { label: string; description: string }`, `interface HitlRequest { id: string; question: string; header: string; options: HitlOption[]; allowTextInput: boolean }`, `interface HitlResponse { id: string; value: string }`, `interface IsolatedSessionInfo { sessionId: SessionId; worktreePath: string; baseCommit: string }`, and the enum string-unions `PauseKind`, `PauseResponse`, `ApprovalChoice`
  #   3. JSON round-trip example: a test instance `let t = SessionTokens { input_tokens: 1024, output_tokens: 512 };` serializes via `serde_json::to_string(&t)` to `{"input_tokens":1024,"output_tokens":512}`, deserializes back to `SessionTokens { input_tokens: 1024, output_tokens: 512 }`, and equality holds — same for every other new type
  #   4. PauseKind round-trip: `serde_json::to_string(&PauseKind::Confirm)` yields `"Confirm"`, `serde_json::to_string(&PauseKind::Triple)` yields `"Triple"`, and deserialization round-trips both variants exactly — proving the enum is wire-portable under the default serde representation
  #   5. HitlRequest round-trip with options: an instance with two HitlOption entries and allow_text_input=true serializes to JSON, deserializes back, and asserts equal — proving Vec<HitlOption> and the boolean field both round-trip cleanly
  #   6. IsolationStateChange additive field: an engineer constructs `StreamChunk::IsolationStateChange { is_isolated: true, worktree_path: Some("/tmp/wt".into()), base_commit: Some("abc1234".into()) }`, serializes to JSON, and the deserialized result equals the original — confirming the new base_commit field is wire-portable and the existing two fields are still present unchanged
  #   7. Cross-frontend usage example: a downstream consumer in codelet/fspec-tui writes `use codelet_rpc_types::{SessionTokens, PauseState, HitlRequest, IsolatedSessionInfo, ThinkingConfig};` and the imports resolve cleanly — proving every new type is publicly re-exported from the rpc-types crate root, matching the existing pattern for WorkUnitInfo, SessionId, and StreamChunk
  #
  # ========================================
  Background: User Story
    As a fspec backend engineer
    I want to add every wire-portable session/agent shape (SessionTokens, TokenRestoreState, SessionModel, WorkUnitContext, ThinkingConfig, PauseKind/PauseState/PauseResponse/ApprovalChoice, HitlOption/HitlRequest/HitlResponse, IsolatedSessionInfo, plus the missing base_commit field on IsolationStateChange) to codelet-rpc-types as the single source of truth
    So that the Rust AgentView and the TypeScript Ink frontend can share identical wire types via the napi feature gate, no AgentView read/write needs to touch napi:: types, and Phase 3 (widening SessionManagerHandle + FspecService) can proceed without latent type duplication

  Scenario: Phase 2.1 per-session derived-state types are added with the documented field shapes
    Given the engineer opens codelet/rpc-types/src/lib.rs after RPC-036 is implemented
    When the engineer searches for the struct declarations SessionTokens, TokenRestoreState, SessionModel, and WorkUnitContext
    Then SessionTokens is declared with exactly two fields, input_tokens: i64 and output_tokens: i64
    And TokenRestoreState is declared with exactly six i64 fields named current_context, cumulative_billed_output, cache_read, cache_creation, cumulative_billed_input, and cumulative_billed_output_second
    And SessionModel is declared with exactly five fields: provider_id: String, model_id: String, context_window: i64, max_output_tokens: i64, compaction_threshold: i64
    And WorkUnitContext is declared with exactly three String fields named id, title, and status
    And each of these four structs derives Debug, Clone, Serialize, Deserialize, and is gated for napi via #[cfg_attr(feature = "napi", napi_derive::napi(object))]

  Scenario: ThinkingConfig holds provider-specific config as a JSON-encoded string
    Given the engineer opens codelet/rpc-types/src/lib.rs after RPC-036 is implemented
    When the engineer reads the ThinkingConfig struct declaration
    Then ThinkingConfig has exactly three fields: provider_id: String, level: ThinkingLevel, config_json: String
    And ThinkingConfig derives Debug, Clone, Serialize, Deserialize
    And ThinkingConfig is gated for napi via #[cfg_attr(feature = "napi", napi_derive::napi(object))]
    And no field of ThinkingConfig uses serde_json::Value, keeping codelet-rpc-types free of any serde_json runtime dependency

  Scenario: Phase 2.2 pause and HITL wire types are added with the AgentView-facing shape
    Given the engineer opens codelet/rpc-types/src/lib.rs after RPC-036 is implemented
    When the engineer searches for the pause and HITL declarations
    Then a PauseKind enum exists with exactly the variants Confirm and Triple, derives Serialize/Deserialize/PartialEq, and is gated for napi via #[cfg_attr(feature = "napi", napi_derive::napi(string_enum))]
    And a PauseState struct exists with exactly kind: PauseKind, prompt: String, tool_call_id: Option<String>
    And a PauseResponse enum exists with exactly the variants Resume, ConfirmAccept, ConfirmDeny, TripleApprove, TripleApproveSession, TripleDeny
    And an ApprovalChoice enum exists with exactly the variants Approve, ApproveSession, Deny
    And a HitlOption struct exists with exactly label: String and description: String
    And a HitlRequest struct exists with exactly id: String, question: String, header: String, options: Vec<HitlOption>, allow_text_input: bool
    And a HitlResponse struct exists with exactly id: String and value: String

  Scenario: Phase 2.4 IsolatedSessionInfo and base_commit augmentation
    Given the engineer opens codelet/rpc-types/src/lib.rs after RPC-036 is implemented
    When the engineer reads the IsolatedSessionInfo struct declaration
    Then IsolatedSessionInfo has exactly three fields: session_id: SessionId, worktree_path: String, base_commit: String
    And the StreamChunk::IsolationStateChange variant has exactly three fields: is_isolated: bool, worktree_path: Option<String>, base_commit: Option<String>
    And every existing StreamChunk variant other than IsolationStateChange is unchanged from the pre-card definition

  Scenario: FspecResult retains its existing byte-compatible shape
    Given the engineer opens codelet/rpc-types/src/lib.rs after RPC-036 is implemented
    When the engineer reads the FspecResult struct declaration
    Then FspecResult has exactly the five pre-card fields: success: bool, data: String, error: Option<String>, system_reminder: Option<String>, tool_call_id: String
    And no field of FspecResult is renamed, reordered, or removed by RPC-036

  Scenario: All new types JSON-round-trip cleanly via serde_json
    Given a test suite under #[cfg(test)] in codelet/rpc-types/src/lib.rs
    And serde_json is declared in codelet/rpc-types/Cargo.toml under [dev-dependencies]
    When the engineer runs `cargo test -p codelet-rpc-types`
    Then every new type (SessionTokens, TokenRestoreState, SessionModel, WorkUnitContext, ThinkingConfig, PauseKind, PauseState, PauseResponse, ApprovalChoice, HitlOption, HitlRequest, HitlResponse, IsolatedSessionInfo) has at least one test that asserts `serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap() == value`
    And StreamChunk::IsolationStateChange has a round-trip test that constructs the variant with base_commit: Some("abc1234"), serializes to JSON, deserializes, and asserts both worktree_path and base_commit are preserved
    And every round-trip test passes

  Scenario: Both feature gates of codelet-rpc-types build cleanly
    Given the engineer is at the workspace root /Users/rquast/projects/fspec/codelet
    When the engineer runs `cargo build -p codelet-rpc-types` with default features
    Then the build succeeds without errors or warnings
    When the engineer runs `cargo build -p codelet-rpc-types --features napi`
    Then the build succeeds without errors or warnings
    And codelet-rpc-types has no dependency on serde_json in its [dependencies] section, only in [dev-dependencies]

  Scenario: codelet-napi continues to compile after the rpc-types widening
    Given the engineer is at the workspace root /Users/rquast/projects/fspec/codelet
    And codelet-napi consumes codelet-rpc-types with the napi feature enabled
    When the engineer runs `cargo build -p codelet-napi` (which transitively enables the napi feature on codelet-rpc-types via its `features = ["napi"]` dep entry — codelet-napi itself does not declare a `napi` feature)
    Then the build succeeds without errors
    And no previously-compiling caller of StreamChunk::isolation_state_change is broken by the additive base_commit field
    And codelet/napi/src/types.rs::stream_chunk_to_json_value destructures the IsolationStateChange variant with the new base_commit field accounted for

  Scenario: New types are publicly re-exported from the rpc-types crate root
    Given a downstream consumer crate
    When the consumer writes `use codelet_rpc_types::{SessionTokens, TokenRestoreState, SessionModel, WorkUnitContext, ThinkingConfig, PauseKind, PauseState, PauseResponse, ApprovalChoice, HitlOption, HitlRequest, HitlResponse, IsolatedSessionInfo};`
    Then every import resolves cleanly, matching the existing public-re-export pattern used for WorkUnitInfo, SessionId, SessionInfo, and StreamChunk
