# RPC-037 — Widen `SessionManagerHandle` + `FspecService` + both backends + stub, with cross-transport parity tests

**Parent:** RPC-030 · **Phase:** 3.1-3.5 · **Estimate:** 13 pts · **Depends on:** RPC-036

## Goal

Extend the trait + tarpc service + both transport backends + the stub so every method the TS frontend calls on `SessionManager` has a peer in pure Rust. After this card, the Rust AgentView can drive any TS-equivalent session action through the `FspecBackend` trait via either embedded or WebSocket transport.

## Current state

### `codelet/core/src/session_manager_handle.rs`

**Existing methods** (default behaviour where noted):
- `list_sessions`, `create_session(role)`, `send_input(text)`, `interrupt`, `get_session_status`, `chunks_rx`, `logs_rx`, `chunks_tx`, `logs_tx`
- `get_model_info` (default `ModelInfo::default()`)
- `get_thinking_level` (default `ThinkingLevel::Off`)
- `list_providers` (default `Vec::new()`)
- `set_model`, `set_thinking_level`, `set_thinking_level_default` (default `Ok(())`)
- `get_role` (default `None`), `set_role` (default `Ok(())`)

### `codelet/rpc/src/lib.rs` — `FspecService` tarpc trait (23 methods)

Existing: `list_work_units`, `list_sessions`, `create_session`, `send_input`, `interrupt`, `get_session_status`, `health`, `checkpoint_counts`, `move_work_unit_up/down`, `get_model_info`, `get_thinking_level`, `get_workspace_info`, `search_files`, `persistence_add_history`, `persistence_get_history`, `persistence_search_history`, `persistence_delete_session`, `list_providers`, `set_session_model`, `set_thinking_level`, `get_session_role`, `set_session_role`.

**Note:** `set_thinking_level_default` exists on `SessionManagerHandle` and `FspecBackend` but NOT on the tarpc trait — close that gap.

### `codelet/fspec-tui/src/transport/`

- `mod.rs` — `FspecBackend` trait (27 methods) + `BackendError`
- `embedded.rs` (`EmbeddedFspecBackend`, 8,209 bytes)
- `websocket.rs` (`WebSocketFspecBackend`, 22,151 bytes)

## What to add

### Step 3.1 — `SessionManagerHandle` trait additions

```rust
// Send-input widened with optional thinking config
fn send_input_with_thinking(
    &self,
    session_id: &SessionId,
    text: String,
    thinking: Option<ThinkingConfig>,
);

// Per-session derived state
fn get_session_tokens(&self, session_id: &SessionId) -> SessionTokens;
fn get_session_model(&self, session_id: &SessionId) -> SessionModel;
fn get_compaction_progress(&self, session_id: &SessionId) -> Option<CompactionProgress>;
fn get_buffered_output(&self, session_id: &SessionId, limit: u32) -> Vec<StreamChunk>;

// Session-history ops
fn clear_history(&self, session_id: &SessionId) -> Result<(), String>;
fn compact_session(&self, session_id: &SessionId) -> Result<CompactionResult, String>;

// Resume/restore
fn restore_session_messages(&self, session_id: &SessionId, envelopes: Vec<String>) -> Result<(), String>;
fn restore_session_token_state(&self, session_id: &SessionId, state: TokenRestoreState) -> Result<(), String>;

// Work-unit binding
fn get_work_unit_context(&self, session_id: &SessionId) -> Option<WorkUnitContext>;
fn set_work_unit_context(&self, session_id: &SessionId, ctx: Option<WorkUnitContext>) -> Result<(), String>;

// Per-session draft text
fn get_pending_input(&self, session_id: &SessionId) -> Option<String>;
fn set_pending_input(&self, session_id: &SessionId, text: Option<String>);

// Active session tracking
fn set_active_session(&self, session_id: &SessionId);
fn clear_active_session(&self);
fn get_active_session(&self) -> Option<SessionId>;

// Effective cwd (isolation-aware)
fn get_effective_cwd(&self, session_id: &SessionId) -> std::path::PathBuf;

// Supervisor links
fn get_supervisors(&self, session_id: &SessionId) -> Vec<SessionId>;

// Debug capture
fn get_debug_enabled(&self, session_id: &SessionId) -> bool;
fn set_debug_enabled(&self, session_id: &SessionId, enabled: bool);
fn toggle_debug(&self, session_id: &SessionId, debug_dir: &str) -> Result<String, String>;

// Pause / HITL
fn pause_resume(&self, session_id: &SessionId) -> Result<(), String>;
fn pause_confirm(&self, session_id: &SessionId, accept: bool) -> Result<(), String>;
fn pause_triple(&self, session_id: &SessionId, choice: ApprovalChoice) -> Result<(), String>;
fn send_hitl_response(&self, session_id: &SessionId, response: HitlResponse) -> Result<(), String>;
fn get_pause_state(&self, session_id: &SessionId) -> Option<PauseState>;
fn get_hitl_request(&self, session_id: &SessionId) -> Option<HitlRequest>;

// FspecCommandRequest round-trip
fn send_fspec_result(&self, session_id: &SessionId, result: FspecResult) -> Result<(), String>;

// Isolation-aware create
fn create_isolated_session(&self, role: Option<String>) -> Result<IsolatedSessionInfo, String>;

// Push-driven status broadcast
fn status_changes_rx(&self) -> tokio::sync::broadcast::Receiver<(SessionId, SessionStatus)>;
fn status_changes_tx(&self) -> tokio::sync::broadcast::Sender<(SessionId, SessionStatus)>;

// Session destruction
fn destroy_session(&self, session_id: &SessionId) -> Result<(), String>;
```

Each method gets a sensible default (e.g. `Vec::new()`, `None`, `Ok(())`, `SessionTokens::default()`) so existing handles compile unchanged.

### Step 3.2 — `StubSessionManagerHandle`

Mirror every new method with a deterministic stub behaviour:
- `get_session_tokens` → return seeded value from `Arc<Mutex<HashMap<SessionId, SessionTokens>>>`
- `clear_history` → `Ok(())` + emit `StreamChunk::user_notification("history cleared")`
- `compact_session` → return canned `CompactionResult { compression_ratio: 0.5, ... }`
- `pause_*` → set internal pause flag + emit `SessionStateChange { state: Paused }`
- `status_changes_tx/rx` → new internal `broadcast::Sender<(SessionId, SessionStatus)>` channel

The stub must be deterministic so the parity tests in Step 3.5 produce identical output across both transports.

### Step 3.3 — `FspecService` tarpc additions

Every trait method gets a matching `async fn` on the tarpc service. Names and signatures stay in lockstep with `SessionManagerHandle` (modulo `async` + `Context` first arg). `FspecServiceImpl` delegates to `self.inner.session_manager()`, falling back to safe defaults when no handle is set.

### Step 3.4 — Both transport backends

- `EmbeddedFspecBackend` (in `codelet/fspec-tui/src/transport/embedded.rs`) — direct method call on `Arc<dyn SessionManagerHandle>`.
- `WebSocketFspecBackend` (in `codelet/fspec-tui/src/transport/websocket.rs`) — tarpc client call.

The `FspecBackend` trait in `transport/mod.rs` gains a method per RPC. Use `#[async_trait]`.

### Step 3.5 — Cross-transport parity tests

Extend `codelet/rpc-embedded/tests/` and `codelet/rpc-server/tests/` with a single parameterised test module that:

1. Constructs `StubSessionManagerHandle`.
2. Runs the same scenario through `EmbeddedFspecBackend` AND `WebSocketFspecBackend`.
3. Asserts the resulting `StreamChunk` sequences and method-return values are byte-identical (modulo timestamps).

Scenarios: every new method exactly once, plus a happy-path "send input → receive Text → Done" round-trip.

## Acceptance criteria

1. Every method in the list above exists on `SessionManagerHandle`, `StubSessionManagerHandle` (deterministic), `FspecService`, `FspecServiceImpl`, `FspecBackend`, `EmbeddedFspecBackend`, `WebSocketFspecBackend`.
2. `cargo build` of `codelet-core`, `codelet-rpc`, `codelet-rpc-types`, `codelet-fspec-tui` all pass.
3. Cross-transport parity test passes for every new method.
4. No regressions: existing tests in `codelet/rpc-embedded/tests/` and `codelet/rpc-server/tests/` stay green.
5. `cargo clippy -p codelet-core -- -D warnings` clean.

## Risks

- `status_changes_rx` is a new broadcast channel. Make sure it survives the embedded→broadcast→websocket→broadcast→embedded round-trip without lag (use `broadcast::Sender::lag()` tracking already implemented in `SharedFspecService`).
- `restore_session_messages(envelopes: Vec<String>)` passes raw JSONL lines — keep that format documented because TS callers serialise their own envelopes.
- HITL `HitlResponse.value` is a free-form string. Don't constrain it.

## Out of scope

- Implementing the methods on a real `SessionManager` → RPC-042.
- Wiring them into the AgentView UI → RPC-045 onwards.
