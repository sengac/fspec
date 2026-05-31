@done
@codelet
@session-management
@infrastructure
@rpc
@RPC-042
Feature: Implement SessionManagerHandle for the extracted SessionManager
  """
  The impl block is placed in `codelet/sessions/src/session_manager.rs` immediately after the existing `impl SessionManager { ... }` block. This co-locates the impl with the struct so future maintainers find it next to the methods it delegates to. Alternatively, if the file approaches the 300-line agent guideline limit (it already is ~982 LOC), the impl block goes in a sibling `codelet/sessions/src/handle_impl.rs` re-exported via `pub mod handle_impl;` from `lib.rs`. The shape test accepts EITHER placement to avoid lock-in.
  The `conversions.rs` module bridges the two parallel pause type families: `codelet_tools::tool_pause::{PauseKind, PauseResponse, PauseState}` (internal, includes `Continue` and `Resumed/Interrupted/Approved/Denied/AllowOnce/AllowSession`) and `codelet_rpc_types::{PauseKind, PauseResponse, PauseState, ApprovalChoice}` (wire-portable, intentionally narrower per the rpc-types comment that says `Continue` is omitted). The `From<tool_pause::PauseState> for rpc_types::PauseState` impl folds Continue→Confirm (loop-control treated as a confirm dialog), preserves Confirm and Triple, and concatenates `tool_name + message` into `prompt` because the wire shape carries one display string while the internal shape carries two.
  End-goal context: this card unblocks RPC-043 (reduce codelet-napi to thin adapter — the napi side keeps calling SessionManager methods directly, but the codelet-fspec binary uses the trait) and RPC-044 (wire codelet_sessions::SessionManager into codelet-fspec::common::build_service). RPC-044 will be a 3-pointer that just calls `Arc::new(SessionManager::new()) as Arc<dyn SessionManagerHandle>` and adds the no-napi dependency regression test. Without this card, the fspec binary cannot inject a real session manager into its FspecService — the trait is the contract. Subsequent slash-command cards (RPC-045..RPC-068) then wire each method through `FspecBackend` to the AgentView store, where most of the heavy lifting lives. Method semantics deferred here (full compaction, full debug capture, full /resume restore) are reclaimed by the named follow-up cards.
  Test pattern follows RPC-039/RPC-040/RPC-041 review-findings precedent: shape tests via grep/substring inspection of source files for static structural assertions; functional tests via real `SessionManager::new()` construction + `Arc<dyn SessionManagerHandle>` casting + per-method round-trips against an empty manager. The functional test file `handle_impl.rs` uses `#[tokio::test(flavor = "multi_thread")]` because some trait methods (`create_session`, `create_isolated_session`) panic without a runtime via `Handle::current()`. The shape test file `handle_impl_shape.rs` uses plain `#[test]` because it only inspects source bytes.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. An `impl codelet_core::SessionManagerHandle for codelet_sessions::SessionManager { ... }` block exists in `codelet/sessions/src/session_manager.rs` (or a sibling module re-exported from `codelet/sessions/src/lib.rs`) and explicitly overrides EVERY method declared on `SessionManagerHandle` — list_sessions, create_session, send_input, send_input_with_thinking, interrupt, get_session_status, chunks_rx, logs_rx, chunks_tx, logs_tx, status_changes_rx, status_changes_tx, get_model_info, get_thinking_level, list_providers, set_model, set_thinking_level, set_thinking_level_default, get_role, set_role, get_session_tokens, get_session_model, get_compaction_progress, get_buffered_output, clear_history, compact_session, restore_session_messages, restore_session_token_state, get_work_unit_context, set_work_unit_context, get_pending_input, set_pending_input, set_active_session, clear_active_session, get_active_session, get_effective_cwd, get_supervisors, get_debug_enabled, set_debug_enabled, toggle_debug, pause_resume, pause_confirm, pause_triple, send_hitl_response, get_pause_state, get_hitl_request, send_fspec_result, create_isolated_session, destroy_session — so the default bodies on the trait are NEVER reached for the real handle. Compile-time proof: `cargo build -p codelet-sessions` succeeds AND a sentinel test casts `Arc::new(SessionManager::new()) as Arc<dyn codelet_core::SessionManagerHandle>` without compile errors.
  #   2. A free helper `fn uuid_from(id: &codelet_rpc_types::SessionId) -> uuid::Uuid` lives alongside the impl block. It parses `id.as_str()` via `Uuid::parse_str` and falls back to `Uuid::nil()` on failure so trait methods that target an unknown / malformed `SessionId` return safe defaults rather than panicking. Every trait method that takes a `&SessionId` uses this helper exactly once at the top of its body (never twice, never inline-duplicated).
  #   3. A new module `codelet/sessions/src/conversions.rs` (re-exported from `codelet/sessions/src/lib.rs`) hosts wire-shape conversions between `codelet_tools::tool_pause::{PauseKind, PauseResponse, PauseState}` and `codelet_rpc_types::{PauseKind, PauseResponse, PauseState}`. Specifically: (a) `impl From<codelet_tools::tool_pause::PauseState> for codelet_rpc_types::PauseState` mapping `kind` (Confirm→Confirm, Triple→Triple, Continue→Confirm fallback because the wire type intentionally omits Continue per rpc-types comment) plus `tool_name+message` → `prompt` concatenation and `details` → `tool_call_id` carry-through; (b) `pub fn approval_choice_to_pause_response(choice: codelet_rpc_types::ApprovalChoice) -> codelet_tools::tool_pause::PauseResponse` mapping Approve→AllowOnce, ApproveSession→AllowSession, Deny→Denied; (c) `pub fn confirm_accept_to_pause_response(accept: bool) -> codelet_tools::tool_pause::PauseResponse` mapping true→Approved, false→Denied. Each conversion is covered by a unit test inside `conversions.rs` (or in `codelet/sessions/tests/conversions.rs`) that asserts every variant pair round-trips correctly.
  #   4. Sync trait methods that need to invoke async `SessionManager` methods (specifically `create_session` and `create_isolated_session`) use `tokio::runtime::Handle::current().block_on(...)` to bridge the sync/async gap. The implementation MUST be documented (Rustdoc comment on the impl block AND the methods) to warn callers that the trait MUST be invoked from a thread with a live tokio runtime — the `fspec` binary always has one. Calling from a non-runtime thread panics with the standard `Handle::current()` panic message — this is intentional and acceptable per the attachment risks section.
  #   5. Per-session methods (`send_input`, `interrupt`, `get_session_status`, `get_session_tokens`, `get_session_model`, `get_compaction_progress`, `get_buffered_output`, `clear_history`, `get_work_unit_context`, `set_work_unit_context`, `get_pending_input`, `set_pending_input`, `get_effective_cwd`, `get_debug_enabled`, `set_debug_enabled`, `toggle_debug`, `pause_resume`, `pause_confirm`, `pause_triple`, `send_hitl_response`, `get_pause_state`, `get_hitl_request`, `send_fspec_result`, `get_role`, `set_role`, `set_thinking_level`, `set_thinking_level_default`, `get_thinking_level`, `get_model_info`, `set_model`, `destroy_session`) consult the session via `self.get_session(&uuid.to_string())`. When the session is NOT found the methods return safe defaults that match the trait-default semantics: `Ok(())` for setters, `SessionStatus::Idle` / `SessionTokens::default()` / `SessionModel { ... zero-filled ... }` for getters, `None` for `Option`-returning getters, `Vec::new()` for `Vec`-returning getters, `Err(format!("Session not found: {}", session_id.as_str()))` for `Result`-returning methods that semantically require a session (`compact_session`, `restore_session_messages`, `restore_session_token_state`, `clear_history`, `set_work_unit_context`, `toggle_debug`, `pause_*`, `send_hitl_response`, `send_fspec_result`, `destroy_session`).
  #   6. Manager-scoped methods (`list_sessions`, `chunks_rx`, `chunks_tx`, `logs_rx`, `logs_tx`, `status_changes_rx`, `status_changes_tx`, `set_active_session`, `clear_active_session`, `get_active_session`, `create_session`, `create_isolated_session`, `list_providers`) delegate directly to existing `SessionManager` methods or its broadcast-sender accessors. Specifically: chunks_rx → `self.chunks_tx().subscribe()`, chunks_tx → `self.chunks_tx().clone()` (logs and status_changes mirror); list_sessions → `SessionManager::list_sessions(self)`; set/clear/get_active_session delegate through the existing `Uuid`-based accessors via `uuid_from`.
  #   7. `compact_session`, `restore_session_messages`, and `restore_session_token_state` are explicitly scoped to a MINIMAL delegating implementation in this card — they look up the session, return `Err("session not found")` when missing, and otherwise return an Ok-with-zero-filled-result (`compact_session` returns the current input-token snapshot for both `original_tokens` and `compacted_tokens` with `compression_ratio: 1.0`; `restore_*` return `Ok(())` no-op). The full agent-loop wiring for `/compact` and `/resume` is deferred to RPC-047 (`/compact` slash command) and RPC-049 (`/resume` durable restore) respectively. Same applies to `toggle_debug`: it flips `set_debug_enabled` on the session, emits `StreamChunk::debug_state_change(new_state)` via the manager-owned `chunks_tx`, and returns `Ok(debug_dir.to_string())` — the full `DebugCaptureManager` start/stop is deferred to RPC-055 (`/debug` debug-capture wiring).
  #   8. A new integration test file `codelet/sessions/tests/handle_impl.rs` exercises the trait impl. The test (a) constructs `SessionManager::new()` with default `NoopSessionManagerHooks`, (b) wraps it in `Arc::new(...) as Arc<dyn codelet_core::SessionManagerHandle>`, (c) drives every trait method at LEAST ONCE against an unknown session-id and asserts the safe-default return value (no panic), (d) for setter/getter pairs that operate on per-session keyed maps inside `BackgroundSession` (pending_input draft, work_unit_context, debug_enabled), it does NOT depend on actually creating a session — those keyed maps live ON `BackgroundSession`, so for an unknown session the get returns None / default and the set is a no-op. The test does NOT exercise `create_session` / `create_isolated_session` end-to-end (they require provider credentials + git + the agent_loop hook) — those paths are exercised by the future RPC-044 integration smoke test in `codelet/fspec/tests/`.
  #   9. A shape-test file `codelet/sessions/tests/handle_impl_shape.rs` enforces the static contract by inspecting `codelet/sessions/src/session_manager.rs` (and any helper modules added by this card) and asserting via grep-style substring matches that (a) `impl codelet_core::SessionManagerHandle for SessionManager` exists, (b) every trait method name from RPC-037 appears as `fn <name>(` inside that impl block, (c) the `fn uuid_from(` helper exists, (d) the `conversions` module is declared (`pub mod conversions;` or similar) and exports `impl From<codelet_tools::tool_pause::PauseState> for codelet_rpc_types::PauseState`. The shape test mirrors the structure of `session_manager_shape.rs` from RPC-040 and `background_session_shape.rs` from RPC-039 so the codebase's existing patterns are preserved.
  #   10. Build invariants ALL hold after the change: `cargo build -p codelet-sessions` succeeds; `cargo build -p codelet-core` succeeds (unchanged); `cargo build -p codelet-napi` continues to succeed because the napi side already re-exports `pub use codelet_sessions::session_manager::SessionManager;` from RPC-040 and the new impl block does not break any existing API; `cargo metadata -p codelet-sessions --format-version 1` continues to report ZERO `codelet-napi` package entries (skeleton_invariants.rs scenario remains green); `cargo clippy -p codelet-sessions --all-targets -- -D warnings` passes (workspace-lints inherited); the regression test `codelet/sessions/tests/skeleton_invariants.rs::scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi` stays green.
  #
  # EXAMPLES:
  #   1. Developer constructs `SessionManager::new()` in a tokio runtime (via `#[tokio::test(flavor = "multi_thread")]`), wraps it in `Arc<dyn SessionManagerHandle>`, calls `handle.list_sessions()` and observes an empty `Vec<SessionInfo>`. No panic, no compile errors.
  #   2. Developer calls every `&SessionId`-taking trait method with `SessionId::new("nonexistent-uuid")` against a fresh manager and observes: `get_session_status` returns `SessionStatus::Idle`; `get_session_tokens` returns `SessionTokens { input_tokens: 0, output_tokens: 0 }`; `get_session_model` returns the zero-filled model; `get_compaction_progress` returns `None`; `get_buffered_output(.., 32)` returns `Vec::new()`; `clear_history` returns `Err(...)` containing "Session not found"; `compact_session` returns `Err(...)` containing "Session not found"; `get_work_unit_context` / `get_pending_input` / `get_pause_state` / `get_hitl_request` / `get_role` return `None`; `get_effective_cwd` returns `std::env::current_dir().unwrap_or_default()`; `get_supervisors` returns `Vec::new()`; `get_debug_enabled` returns `false`; `toggle_debug` returns `Err(...)` containing "Session not found"; setters that take `&SessionId` are silent no-ops (return `Ok(())` for `Result` setters, no panic for unit-returning setters).
  #   3. Developer subscribes to `handle.chunks_rx()`, `handle.logs_rx()`, and `handle.status_changes_rx()` against the manager, then publishes one chunk through `handle.chunks_tx().send((SessionId::new("abc"), StreamChunk::done()))` and one status update through `handle.status_changes_tx().send((SessionId::new("abc"), SessionStatus::Idle))`. Both subscribers observe the published items in arrival order. Logs broadcast is also functional (subscribe + send round-trip).
  #   4. Developer calls `handle.set_active_session(&SessionId::new("abc"))`, then `handle.get_active_session()` returns `Some(SessionId::new("abc"))`. Then `handle.clear_active_session()` and `handle.get_active_session()` returns `None`. This works without any session actually existing because the manager's `active_session_id` field is keyed by Uuid only.
  #   5. Developer inspects `codelet/sessions/src/session_manager.rs` (or the new `handle_impl.rs` sibling module if extracted) and observes: a single `impl codelet_core::SessionManagerHandle for SessionManager { ... }` block containing one `fn <name>(...)` for EVERY method listed in rule [0]; no method delegates only to the trait's default — every override has an explicit body. The body for sync→async bridges (`create_session`, `create_isolated_session`) contains the literal substring `tokio::runtime::Handle::current().block_on(` exactly once.
  #   6. Developer inspects `codelet/sessions/src/conversions.rs` and observes: `impl From<codelet_tools::tool_pause::PauseState> for codelet_rpc_types::PauseState` exists; `pub fn approval_choice_to_pause_response` exists; `pub fn confirm_accept_to_pause_response` exists. Unit tests in the same file (or `codelet/sessions/tests/conversions.rs`) exercise every variant of `codelet_tools::tool_pause::PauseKind` (Continue→Confirm fallback, Confirm→Confirm, Triple→Triple), every `codelet_rpc_types::ApprovalChoice` variant (Approve→AllowOnce, ApproveSession→AllowSession, Deny→Denied), and both `bool` branches for `confirm_accept_to_pause_response`.
  #   7. Developer runs `cargo build -p codelet-sessions` — build succeeds. Then runs `cargo test -p codelet-sessions --test handle_impl --test handle_impl_shape --test conversions` — all three test files pass. Then runs `cargo build -p codelet-napi` — build succeeds because the new impl block does not touch any pre-existing public API and the existing `pub use codelet_sessions::session_manager::SessionManager;` re-export keeps every napi-side caller compiling.
  #   8. Developer runs `cargo metadata -p codelet-sessions --format-version 1 | jq '.packages[].name' | grep '^"codelet-napi"$'` — zero matches. The forbidden arrow `sessions → napi` stays absent. The `skeleton_invariants.rs::scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi` test stays green.
  #
  # ========================================
  Background: User Story
    As a fspec binary or any Rust consumer
    I want to use codelet_sessions::SessionManager as an Arc<dyn codelet_core::SessionManagerHandle>
    So that drive real agent sessions through the trait surface that AgentView and both transports already consume, without depending on codelet-napi

  @rust
  @smoke
  Scenario: SessionManager satisfies the codelet-core SessionManagerHandle trait object
    Given the codelet-sessions crate compiles
    And a tokio multi-threaded runtime is active for the test
    When I construct a fresh "codelet_sessions::SessionManager" via "SessionManager::new()"
    And I cast it via "Arc::new(manager) as Arc<dyn codelet_core::SessionManagerHandle>"
    Then the cast compiles without error
    And calling "handle.list_sessions()" on the trait object returns an empty "Vec<SessionInfo>"

  @rust
  @edge-case
  Scenario: Every per-session method returns the safe trait-default for an unknown SessionId
    Given a fresh "SessionManager" wrapped as "Arc<dyn SessionManagerHandle>"
    And a "SessionId::new(\"nonexistent-uuid\")" that is NOT registered in the manager
    When I call every per-session method with that "SessionId"
    Then "get_session_status" returns "SessionStatus::Idle"
    And "get_session_tokens" returns "SessionTokens { input_tokens: 0, output_tokens: 0 }"
    And "get_session_model" returns the zero-filled "SessionModel"
    And "get_compaction_progress" returns "None"
    And "get_buffered_output(.., 32)" returns "Vec::new()"
    And "get_work_unit_context" returns "None"
    And "get_pending_input" returns "None"
    And "get_pause_state" returns "None"
    And "get_hitl_request" returns "None"
    And "get_role" returns "None"
    And "get_effective_cwd" returns a non-empty "PathBuf" (the process cwd fallback)
    And "get_supervisors" returns "Vec::new()"
    And "get_debug_enabled" returns "false"
    And "clear_history", "compact_session", "toggle_debug", "pause_resume", "pause_confirm", "pause_triple", "send_hitl_response", "send_fspec_result", "destroy_session", "restore_session_messages", "restore_session_token_state", "set_work_unit_context", "set_thinking_level", "set_thinking_level_default", "set_role", "set_model" all return "Err(...)" containing the substring "Session not found"
    And "set_pending_input", "set_debug_enabled", "set_active_session", "interrupt", "send_input", "send_input_with_thinking" do NOT panic

  @rust
  @parity
  @streaming
  Scenario: Broadcast accessors round-trip chunks, logs, and status updates
    Given a fresh "SessionManager" wrapped as "Arc<dyn SessionManagerHandle>"
    When I subscribe via "handle.chunks_rx()", "handle.logs_rx()", and "handle.status_changes_rx()"
    And I publish "(SessionId::new(\"abc\"), StreamChunk::done())" via "handle.chunks_tx().send(...)"
    And I publish "(SessionId::new(\"abc\"), SessionStatus::Idle)" via "handle.status_changes_tx().send(...)"
    And I publish a "LogRecord" via "handle.logs_tx().send(...)"
    Then every subscriber observes the published item in arrival order
    And no broadcast lag or "Closed" error is observed for a single-item publish

  @rust
  @state-management
  Scenario: Active session tracking is manager-scoped and works without a real session row
    Given a fresh "SessionManager" wrapped as "Arc<dyn SessionManagerHandle>"
    When I call "handle.set_active_session(&SessionId::new(\"00000000-0000-0000-0000-000000000001\"))"
    Then "handle.get_active_session()" returns "Some(SessionId::new(\"00000000-0000-0000-0000-000000000001\"))"
    When I call "handle.clear_active_session()"
    Then "handle.get_active_session()" returns "None"

  @rust
  @source-shape
  Scenario: The impl block exists with explicit overrides for every trait method
    Given the file "codelet/sessions/src/session_manager.rs" (and any sibling "handle_impl.rs" if used)
    When I read the source bytes and scan them
    Then exactly one "impl codelet_core::SessionManagerHandle for SessionManager" block exists across the inspected files
    And the impl block contains a "fn list_sessions(" override
    And the impl block contains a "fn create_session(" override
    And the impl block contains a "fn send_input(" override
    And the impl block contains a "fn send_input_with_thinking(" override
    And the impl block contains a "fn interrupt(" override
    And the impl block contains a "fn get_session_status(" override
    And the impl block contains a "fn chunks_rx(" override
    And the impl block contains a "fn logs_rx(" override
    And the impl block contains a "fn chunks_tx(" override
    And the impl block contains a "fn logs_tx(" override
    And the impl block contains a "fn status_changes_rx(" override
    And the impl block contains a "fn status_changes_tx(" override
    And the impl block contains a "fn get_session_tokens(" override
    And the impl block contains a "fn get_session_model(" override
    And the impl block contains a "fn get_compaction_progress(" override
    And the impl block contains a "fn get_buffered_output(" override
    And the impl block contains a "fn clear_history(" override
    And the impl block contains a "fn compact_session(" override
    And the impl block contains a "fn restore_session_messages(" override
    And the impl block contains a "fn restore_session_token_state(" override
    And the impl block contains a "fn get_work_unit_context(" override
    And the impl block contains a "fn set_work_unit_context(" override
    And the impl block contains a "fn get_pending_input(" override
    And the impl block contains a "fn set_pending_input(" override
    And the impl block contains a "fn set_active_session(" override
    And the impl block contains a "fn clear_active_session(" override
    And the impl block contains a "fn get_active_session(" override
    And the impl block contains a "fn get_effective_cwd(" override
    And the impl block contains a "fn get_supervisors(" override
    And the impl block contains a "fn get_debug_enabled(" override
    And the impl block contains a "fn set_debug_enabled(" override
    And the impl block contains a "fn toggle_debug(" override
    And the impl block contains a "fn pause_resume(" override
    And the impl block contains a "fn pause_confirm(" override
    And the impl block contains a "fn pause_triple(" override
    And the impl block contains a "fn send_hitl_response(" override
    And the impl block contains a "fn get_pause_state(" override
    And the impl block contains a "fn get_hitl_request(" override
    And the impl block contains a "fn send_fspec_result(" override
    And the impl block contains a "fn create_isolated_session(" override
    And the impl block contains a "fn destroy_session(" override
    And the impl block contains a "fn set_thinking_level_default(" override
    And a "fn uuid_from(" helper exists alongside the impl
    And the source contains exactly one occurrence of "tokio::runtime::Handle::current().block_on(" across the create_session and create_isolated_session overrides (counted across BOTH methods)

  @rust
  @source-shape
  @pause-integration
  Scenario: The conversions module bridges tool_pause and rpc-types pause families
    Given the file "codelet/sessions/src/conversions.rs"
    And the module is declared from "codelet/sessions/src/lib.rs" via "pub mod conversions;"
    When I read the source bytes
    Then the file contains "pub fn pause_state_to_rpc"
    And the file contains "pub fn approval_choice_to_pause_response"
    And the file contains "pub fn confirm_accept_to_pause_response"

  @rust
  @pause-integration
  @unit
  Scenario: Conversion helpers map every variant of tool_pause to its rpc-types peer
    Given the conversions module is reachable as "codelet_sessions::conversions"
    When I call "pause_state_to_rpc(tool_pause::PauseState { kind: Continue, .. })"
    Then the resulting "rpc_types::PauseState.kind" equals "rpc_types::PauseKind::Confirm"
    When I call "pause_state_to_rpc(tool_pause::PauseState { kind: Confirm, .. })"
    Then the resulting "rpc_types::PauseState.kind" equals "rpc_types::PauseKind::Confirm"
    When I call "pause_state_to_rpc(tool_pause::PauseState { kind: Triple, .. })"
    Then the resulting "rpc_types::PauseState.kind" equals "rpc_types::PauseKind::Triple"
    When I call "approval_choice_to_pause_response(ApprovalChoice::Approve)"
    Then the result is "tool_pause::PauseResponse::AllowOnce"
    When I call "approval_choice_to_pause_response(ApprovalChoice::ApproveSession)"
    Then the result is "tool_pause::PauseResponse::AllowSession"
    When I call "approval_choice_to_pause_response(ApprovalChoice::Deny)"
    Then the result is "tool_pause::PauseResponse::Denied"
    When I call "confirm_accept_to_pause_response(true)"
    Then the result is "tool_pause::PauseResponse::Approved"
    When I call "confirm_accept_to_pause_response(false)"
    Then the result is "tool_pause::PauseResponse::Denied"

  @rust
  @build
  @regression
  Scenario: All build and dependency-rule invariants remain green
    Given the RPC-042 changes are applied to the working tree
    When I run "cargo build -p codelet-sessions"
    Then the build succeeds
    When I run "cargo build -p codelet-core"
    Then the build succeeds
    When I run "cargo build -p codelet-napi"
    Then the build succeeds
    When I run "cargo metadata -p codelet-sessions --format-version 1"
    Then the reported package list contains ZERO entries equal to "codelet-napi"
    When I run "cargo test -p codelet-sessions --test skeleton_invariants"
    Then the test "scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi" passes
