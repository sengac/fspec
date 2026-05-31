# Roadmap: Port BackgroundSession + Agent Management to the Rust Frontend

**Card:** RPC-030
**Parent / continues from:** RPC-029 (AgentView structural alignment)
**Goal:** A 100% correct port of the TypeScript Ink AgentView's session & agent-management surface to the Rust ratatui frontend, with `codelet-napi` reduced to a thin adapter and **zero `napi → rpc` and `rpc → napi` arrows**.

---

## The single architectural rule

```
fspec (binary) ──► fspec-tui ──► rpc / rpc-embedded / rpc-server ──► core ──► providers / tools / git / cli / common
                                                                       ▲
                                                                       │
                                                            codelet-napi (sink — nothing imports out of it)
```

- `rpc → napi`: **forbidden**, enforced by `codelet/rpc-embedded/tests/rpc_006_source_shape.rs`.
- `fspec → napi`: **forbidden** by absence (no entry in `codelet/fspec/Cargo.toml`).
- `napi` becomes a pure adapter over the same Rust types both frontends consume.
- All cross-frontend types live in `codelet-rpc-types`.
- All cross-frontend behaviour goes through `codelet-core::SessionManagerHandle` trait.

Everything below is the work required to reach that target state.

---

## Phase 1 — Lift the persistence layer out of NAPI into `codelet-core::persistence`

This is the first work in the chain. Nothing else can be durable until this is done. All on-disk JSONL formats remain byte-identical (RPC-025 / RPC-026 set the precedent).

### Step 1.1 — Lift `MessageEnvelope` types
Move `MessageEnvelope`, `MessagePayload`, `UserMessage`, `UserContent`, `AssistantMessage`, `AssistantContent` from `codelet/napi/src/persistence/message_envelope.rs` (~26 kB) into `codelet-core::persistence::message_envelope`. NAPI re-exports.

### Step 1.2 — Lift `MessageStore`
Move `MessageStore` + `message_index.rs` (~33 kB combined) into `codelet-core::persistence::messages`.

### Step 1.3 — Lift `SessionStore` (manifest)
Move `SessionStore`, `load_session(uuid) -> SessionManifest`, `append_message`, `append_message_with_metadata`, `update_session_tokens` from `codelet/napi/src/persistence/mod.rs` (~24 kB) into `codelet-core::persistence::manifest`.

### Step 1.4 — Lift `BlobStore`
Move `blob.rs` + `blob_processing.rs` (~15 kB combined) into `codelet-core::persistence::blob`.

### Step 1.5 — Update `codelet-napi` to re-export
Every existing `#[napi]` persistence function in `codelet/napi/src/persistence/napi_bindings.rs` becomes a thin shim that calls into `codelet-core::persistence`.

**Exit criterion:** `codelet/napi/src/persistence/` contains ONLY `napi_bindings.rs` and any per-NAPI test helpers. All pure Rust persistence logic is in `codelet-core::persistence`.

---

## Phase 2 — Widen `codelet-rpc-types` with every wire-portable shape AgentView needs

Add these structs/enums to `codelet/rpc-types/src/lib.rs` (all with `#[cfg_attr(feature = "napi", napi_derive::napi(object))]` and `Serialize + Deserialize`):

### Step 2.1 — Per-session derived state
- `SessionTokens { input_tokens: i64, output_tokens: i64 }`
- `TokenRestoreState { current_context: i64, cumulative_billed_output: i64, cache_read: i64, cache_creation: i64, cumulative_billed_input: i64, cumulative_billed_output_second: i64 }`
- `SessionModel { provider_id: String, model_id: String, context_window: i64, max_output_tokens: i64, compaction_threshold: i64 }`
- `CompactionProgress { phase: String, current: i64, total: i64 }`
- `CompactionResult { compression_ratio: f64, original_tokens: i64, compacted_tokens: i64, turns_summarized: i64, turns_kept: i64 }`
- `WorkUnitContext { id: String, title: String, status: String }`
- `ThinkingConfig` (provider-specific JSON-shaped struct; matches what `getThinkingConfig(providerId, level)` produces)

### Step 2.2 — Pause & HITL
- `PauseKind` enum (Confirm, Triple)
- `PauseState { kind: PauseKind, prompt: String, tool_call_id: Option<String> }`
- `PauseResponse` enum (Resume, ConfirmAccept, ConfirmDeny, TripleApprove, TripleApproveSession, TripleDeny)
- `ApprovalChoice` enum (Approve, ApproveSession, Deny)
- `HitlRequest { question: String, header: String, options: Vec<HitlOption>, ... }`
- `HitlResponse { id: String, value: String }`
- `HitlOption { label: String, description: String }`

### Step 2.3 — `StreamChunk` variants for the missing chunk types
Add to the `StreamChunk` enum:
- `SessionStateChange { status: SessionStatus }` (parity with NAPI emission on every status set)
- `IsolationStateChange { worktree_path: String, base_commit: String, is_isolated: bool }`
- `DebugStateChange { enabled: bool }`
- `FooterStateUpdate { ... }` (footer chrome data)
- `FspecCommandRequest { tool_call_id: String, command: String, args: serde_json::Value }`

### Step 2.4 — Supporting types
- `FspecResult { success: bool, data: Option<serde_json::Value>, error: Option<String>, system_reminder: Option<String>, tool_call_id: String }`
- `IsolatedSessionInfo { session_id: SessionId, worktree_path: String, base_commit: String }`

**Exit criterion:** Every data shape that crosses the AgentView ↔ session-manager boundary on the JS side has a peer in `codelet-rpc-types`. Nothing AgentView reads/writes touches `napi::` types.

---

## Phase 3 — Widen the `SessionManagerHandle` trait + the `FspecService` RPC surface

Extend `codelet/core/src/session_manager_handle.rs` with every method the JS frontend reaches for. Each method gets a sensible default in the trait so existing handles (`StubSessionManagerHandle` and any future handles) compile unchanged. Every trait method gets a matching method on `FspecService` in `codelet/rpc/src/lib.rs` that routes through `self.inner.session_manager()`.

### Step 3.1 — Trait additions

```rust
// Send-input widened to carry thinking config
fn send_input(&self, session_id: &SessionId, text: String, thinking: Option<ThinkingConfig>);

// Per-session derived state reads
fn get_session_tokens(&self, session_id: &SessionId) -> SessionTokens;
fn get_session_model(&self, session_id: &SessionId) -> SessionModel;
fn get_compaction_progress(&self, session_id: &SessionId) -> Option<CompactionProgress>;
fn get_buffered_output(&self, session_id: &SessionId, limit: u32) -> Vec<StreamChunk>;

// Session-history operations
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

// Pause / HITL responder
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

// Per-user default thinking level
fn set_thinking_level_default(&self, level: ThinkingLevel) -> Result<(), String>;

// Push-driven status broadcast (replaces polling)
fn status_changes_rx(&self) -> tokio::sync::broadcast::Receiver<(SessionId, SessionStatus)>;

// Session destruction
fn destroy_session(&self, session_id: &SessionId) -> Result<(), String>;
```

### Step 3.2 — Mirror every method on `StubSessionManagerHandle`
Each addition gets a deterministic stub so cross-transport parity tests keep passing.

### Step 3.3 — Mirror every method on `FspecService`
Each trait method gets a matching `async fn` on the tarpc service with a routed-through-handle implementation in `FspecServiceImpl`. Default-when-no-handle branches return safe defaults (`Vec::new()`, `Ok(())`, `None`, `SessionTokens::default()`).

### Step 3.4 — Update both backends
- `EmbeddedFspecBackend` (in `codelet/fspec-tui/src/transport/embedded.rs`) gains a method per RPC.
- `WebSocketFspecBackend` (in `codelet/fspec-tui/src/transport/websocket.rs`) gains a method per RPC.
- A single trait `FspecBackend` declares the surface so AgentView doesn't care which transport is live.

### Step 3.5 — Cross-transport parity test
Extend `codelet/rpc-embedded/tests/` and `codelet/rpc-server/tests/` so every new method is exercised against the stub through both transports with identical assertions.

**Exit criterion:** Every method JS calls on the session manager exists on `SessionManagerHandle`, on `FspecService`, on both backends, on the stub, and is tested through both transports.

---

## Phase 4 — Extract `SessionManager` + `BackgroundSession` into a new NAPI-free crate

Create `codelet/sessions/` (`codelet-sessions`). It owns the full agent loop, replacing the current placement under `codelet/napi/src/session_manager.rs`. `codelet-napi` becomes a thin adapter.

### Step 4.1 — Create the new crate
- New `codelet/sessions/Cargo.toml`.
- Workspace dependencies: `codelet-common`, `codelet-tools`, `codelet-providers`, `codelet-cli`, `codelet-git`, `codelet-core`, `codelet-rpc-types` (no `napi` feature).
- Add to root `Cargo.toml` workspace members.

### Step 4.2 — Move `BackgroundSession` verbatim
- Move the entire `BackgroundSession` struct + impl from `codelet/napi/src/session_manager.rs` into `codelet/sessions/src/background_session.rs`.
- Replace `napi::Error::from_reason(...)` in `send_input` with `String` errors. This is the one `napi::` reference inside the type.
- Replace `crate::persistence::{...}` imports with `codelet_core::persistence::{...}` (Phase 1 prerequisite).
- Replace `crate::types::{...}` (NAPI-local types) with `codelet_rpc_types::{...}` (Phase 2 prerequisite).

### Step 4.3 — Move `SessionManager` verbatim
- Move the `SessionManager` struct + impl into `codelet/sessions/src/session_manager.rs`.
- All `#[napi]` attributes removed.
- `SessionManager` exposes plain Rust methods (no JS-facing return types).

### Step 4.4 — Replace `GLOBAL_CHUNK_CALLBACK` with a `tokio::sync::broadcast` sender
- `SessionManager` owns `chunks_tx: broadcast::Sender<(SessionId, StreamChunk)>` and `logs_tx: broadcast::Sender<LogRecord>`.
- `BackgroundSession::handle_output` calls `chunks_tx.send((session_id, chunk))` instead of `GLOBAL_CHUNK_CALLBACK.call(...)`.
- Drop the `OnceCell<GlobalChunkCallback>` global entirely.
- Drop `unsafe impl Send/Sync for GlobalChunkCallback`.

### Step 4.5 — Implement `SessionManagerHandle` for the new `SessionManager`
- In `codelet/sessions/src/lib.rs`, add `impl codelet_core::SessionManagerHandle for SessionManager { ... }` covering every trait method from Phase 3.
- Each impl delegates to the corresponding `BackgroundSession` method via `self.sessions.read().get(session_id)`.

### Step 4.6 — Reduce `codelet-napi` to a thin adapter
- Delete `codelet/napi/src/session_manager.rs` (all the agent logic moved out).
- New thinner file (`codelet/napi/src/session_bindings.rs`, ~500 LOC target) that:
  - Holds an `Arc<codelet_sessions::SessionManager>` singleton.
  - Subscribes once at startup to `chunks_tx` (broadcast) and fans into the JS `ThreadsafeFunction` so JS keeps its existing `sessionSetGlobalChunkCallback` API.
  - Wraps every public `SessionManager` / `BackgroundSession` method behind a `#[napi]` function that does ONLY type conversion (Rust types → NAPI-bridge types).
- `codelet-napi` continues to expose the same TS-facing API surface (verified by `codelet/napi/index.d.ts` regression).

### Step 4.7 — Wire `codelet-napi` into the new crate
- `codelet/napi/Cargo.toml` adds `codelet-sessions = { path = "../sessions" }`.
- Remove the old direct dependencies on `codelet-cli` etc. that are now reached through `codelet-sessions`.

**Exit criterion:**
- `codelet/sessions/` compiles standalone with no `napi` dependency.
- `codelet-napi` is a thin adapter (no agent-loop logic).
- TS frontend behaviour is unchanged (regression-tested via the existing TS test suite).
- `GLOBAL_CHUNK_CALLBACK` is deleted.

---

## Phase 5 — Wire the extracted `SessionManager` into the `fspec` binary

### Step 5.1 — Build the manager in `codelet-fspec`
Edit `codelet/fspec/src/common.rs::build_service`:

```rust
pub fn build_service(workspace: &Path) -> Result<Arc<SharedFspecService>> {
    let watcher = Arc::new(WorkUnitsWatcher::new(workspace.to_path_buf())?);
    let session_manager: Arc<dyn SessionManagerHandle> = Arc::new(
        codelet_sessions::SessionManager::new(workspace.to_path_buf())?,
    );
    Ok(Arc::new(
        SharedFspecService::with_session_manager(watcher, session_manager)
            .with_cwd(workspace.to_path_buf()),
    ))
}
```

### Step 5.2 — Add `codelet-sessions` to `codelet-fspec` Cargo.toml
- `codelet-sessions = { path = "../sessions" }`.
- No `codelet-napi` entry. The forbidden arrow stays absent.

### Step 5.3 — Validate the dependency rule
- Run `codelet/rpc-embedded/tests/rpc_006_source_shape.rs` — it must continue to assert no `rpc → napi` references.
- Add an equivalent test asserting no `fspec → napi` references.
- Add a test asserting no `fspec-tui → napi` references.

**Exit criterion:** `fspec` (combined, daemon, client modes) all run real agent sessions via the extracted `SessionManager`. No NAPI dependency anywhere in the `fspec` binary's transitive graph.

---

## Phase 6 — Wire the AgentView to the new RPC surface (TS-parity)

The Rust AgentView is already structurally correct (RPC-029). Now connect every UI action through the widened `FspecBackend` trait.

### Step 6.1 — Subscribe to the chunks broadcast
- In `App::run`'s `tokio::select!`, subscribe to `backend.chunks_rx()` and dispatch every `(SessionId, StreamChunk)` to `App::dispatch` as a `StreamChunkReceived` action.
- The dispatcher routes the chunk into the per-session `SessionContext` scrollback.

### Step 6.2 — Subscribe to the status-changes broadcast
- In `App::run`'s `tokio::select!`, subscribe to `backend.status_changes_rx()`.
- Drop the polling `get_session_status` call from the dispatcher.
- Status pill in `SessionFooter` becomes push-driven.

### Step 6.3 — Handle every new `StreamChunk` variant
For each new variant added in Phase 2.3, the dispatcher (`codelet/fspec-tui/src/views/agent/dispatch.rs` and friends) must update store state:
- `SessionStateChange` → update `SessionContext::status`.
- `IsolationStateChange` → update isolation indicator in `SessionFooter`.
- `DebugStateChange` → update debug badge.
- `FooterStateUpdate` → update footer chrome state.
- `FspecCommandRequest` → execute the requested CLI command via the Rust-side command runner, build a `FspecResult`, call `backend.send_fspec_result(session_id, result)`.

### Step 6.4 — Wire every TS-equivalent slash command

For every command in `src/tui/utils/slashCommands.ts`, the Rust registry in `codelet/fspec-tui/src/views/agent/slash_commands.rs` must dispatch the equivalent action.

| Command | Dispatch chain |
|---|---|
| `/model` | `ModelSelectorDialog` opens → on select: `backend.set_session_model(session_id, provider_id, model_id)` |
| `/provider` | New child view `ProviderSettingsView`. Builds on `backend.list_providers()` plus a new provider-credentials RPC surface (see Phase 7) |
| `/debug` | `backend.toggle_debug(session_id, debug_dir)` |
| `/clear` | `backend.clear_history(session_id)` |
| `/compact` | `backend.compact_session(session_id)` → display `CompactionResult` in scrollback |
| `/thinking [off\|low\|med\|high]` | Arg-parsed → `backend.set_thinking_level(session_id, level)`; no-arg → `ThinkingLevelDialog` |
| `/resume` | `ResumeSessionView` lists sessions via `backend.list_sessions()` → on select: `backend.restore_session_messages` + `restore_session_token_state` + (re)subscribe |
| `/detach` | `backend.set_work_unit_context(session_id, None)` |
| `/search` | `SearchHistoryView` calls `backend.persistence_search_history(query)` |
| `/blocklist` | New child view `BlocklistView` (Phase 7) |
| `/role` | `RoleDialog` reads via `backend.get_session_role` / writes via `backend.set_session_role` |
| `/merge-worktree` | New worktree-merge flow (Phase 7) |
| `/schedule` | New scheduler subcommand handler (Phase 7) |
| `/loop` | New loop subcommand handler (Phase 7) |

### Step 6.5 — Wire keyboard shortcuts to parity
- `Shift+←/→` session navigation: already wired to `AgentViewStore::cycle_session`.
- `Shift+↑/↓` history: wire to `backend.persistence_get_history`.
- `Tab` turn-selection mode: already wired.
- `Esc` priority cascade: extend to call `backend.interrupt(session_id)` when running.
- `Ctrl+R` history search: open `SearchHistoryView`.

### Step 6.6 — Wire the work-unit attach path
- BoardView click on a work unit dispatches `AttachSessionToWorkUnit(work_unit_id)`.
- Dispatcher calls `backend.set_work_unit_context(session_id, Some(ctx))`.
- `SessionHeader` renders the work-unit chip from `backend.get_work_unit_context(session_id)`.

### Step 6.7 — Wire the pending-input draft
- `MultiLineInput` calls `backend.set_pending_input(session_id, text)` on every change (debounced).
- `App::dispatch` calls `backend.get_pending_input(session_id)` on session activation and seeds the input.

### Step 6.8 — Wire pause / HITL UI
- `ConfirmDialog` reads `backend.get_pause_state(session_id)`.
- Two-choice confirm: `backend.pause_confirm(session_id, accept)`.
- Three-choice confirm: `backend.pause_triple(session_id, choice)`.
- Resume: `backend.pause_resume(session_id)`.
- HITL dialog: `backend.get_hitl_request(session_id)` → on submit: `backend.send_hitl_response(session_id, response)`.

**Exit criterion:** Every slash command, dialog, keyboard shortcut, chunk variant, and state read in the TS AgentView has a 1:1 working equivalent on the Rust AgentView, routed through the `FspecBackend` trait.

---

## Phase 7 — Port the remaining subsystems used by slash commands

### Step 7.1 — Provider settings (`/provider`)
- New child view `ProviderSettingsView` in `codelet/fspec-tui/src/views/`.
- New RPC methods on `FspecService`: `list_provider_credentials`, `set_provider_credentials`, `test_provider_connection`, `refresh_models_cache`.
- Trait additions on `SessionManagerHandle` for the same.
- Implementation in `codelet-sessions` delegates to `codelet-providers` (already NAPI-free).

### Step 7.2 — Debug capture (`/debug`)
- `DebugCaptureManager` already lives in `codelet-common::debug_capture` — NAPI-free.
- Wire `backend.toggle_debug` through the trait already defined in Phase 3.3.
- New RPC method: `set_debug_directory(path)` for the pre-session global toggle.

### Step 7.3 — Blocklist (`/blocklist`)
- New child view `BlocklistView`.
- New RPC methods: `blocklist_list`, `blocklist_add`, `blocklist_remove`, `blocklist_update`.
- Trait additions for the same.
- Implementation in `codelet-sessions` delegates to `codelet-tools` blocklist (already NAPI-free).

### Step 7.4 — Worktree merge (`/merge-worktree`)
- New RPC methods: `merge_session_worktree`, `discard_session_worktree`, `prune_orphaned_worktrees`, `list_session_worktrees`, `inspect_session_changes`.
- Trait additions for the same.
- Implementation in `codelet-sessions` delegates to `codelet-git` (already NAPI-free).
- Confirm dialog UI in `codelet/fspec-tui/src/views/agent/` for the merge/discard prompt.

### Step 7.5 — Schedule (`/schedule`)
- Scheduler engine currently in `codelet/napi/src/` — lift the engine logic into `codelet-core::scheduler` (same lift pattern as persistence).
- New RPC methods: `schedule_add`, `schedule_list`, `schedule_pause`, `schedule_resume`, `schedule_remove`.
- Trait additions for the same.

### Step 7.6 — Loop (`/loop`)
- Loop store currently in `codelet/napi/src/` — lift into `codelet-core::loops`.
- New RPC methods: `loop_add`, `loop_cancel`, `loop_list`.
- Trait additions for the same.

### Step 7.7 — Isolated session creation
- `backend.create_isolated_session(role)` returns `IsolatedSessionInfo`.
- Implementation in `codelet-sessions` ports `SessionManager::create_isolated_session_with_id` (calls `codelet-git::create_worktree` + `create_session_manifest`).
- AgentView wires "create isolated" as a variant of the `/new` flow (or a separate slash command if the TS adds one).

### Step 7.8 — Supervisor / subordinate links
- Port the supervisor surface from `BackgroundSession` (WATCH-003/006/008/011/019/020).
- Trait additions: `add_supervisor`, `remove_supervisor`, `get_supervisors`, `get_subordinates`, `receive_incoming_message`.
- AgentView surfaces a "this is a subordinate" badge based on `backend.get_supervisors`.

### Step 7.9 — MCP injection
- `McpInjection` types already live in `codelet-tools` (NAPI-free).
- Port the `mcp_injection_rx` plumbing into the new `SessionManager` (Phase 4.3 already moved it implicitly).
- No new RPC surface needed (purely internal to the agent loop).

**Exit criterion:** Every TS slash command works on the Rust AgentView. Every TS feature reachable from AgentView (provider settings, debug capture, blocklist, worktree merge, schedule, loops, isolation, supervisor links, MCP) has a 1:1 working equivalent.

---

## Phase 8 — Verification

### Step 8.1 — Behaviour parity test suite
For every slash command and every keyboard shortcut, write an integration test in `codelet/fspec-tui/tests/` that drives the AgentView through a deterministic stub backend and asserts the exact same store-state transitions the TS frontend would produce. The TS frontend's test suite is the reference.

### Step 8.2 — Cross-frontend integration test
Boot the `fspec` binary in combined mode against a stub provider. Drive every slash command end-to-end. Capture chunks. Assert the chunk stream matches the equivalent TS-frontend run against the same stub provider, modulo cosmetic differences (timestamps, UUIDs).

### Step 8.3 — Dependency-rule regression tests
- `codelet/rpc-embedded/tests/rpc_006_source_shape.rs` — existing test, must remain green.
- New: `codelet/fspec/tests/no_napi_dependency.rs` — fails the build if `codelet-napi` appears in the transitive graph.
- New: `codelet/fspec-tui/tests/no_napi_dependency.rs` — same.
- New: `codelet/sessions/tests/no_napi_dependency.rs` — same.

### Step 8.4 — TS frontend regression
Run the full TS test suite. Since `codelet-napi` is now a thin adapter over `codelet-sessions`, every TS-facing function must still produce identical behaviour. Any failure here means the Phase 4 extraction broke something.

### Step 8.5 — Boundary audit
- Confirm no `use codelet_napi` exists anywhere in `codelet/{core,rpc,rpc-types,rpc-embedded,rpc-server,fspec,fspec-tui,sessions}`.
- Confirm `GLOBAL_CHUNK_CALLBACK` is deleted.
- Confirm `codelet/napi/src/session_manager.rs` is deleted (replaced by the thin `session_bindings.rs` adapter).
- Confirm `codelet/napi/src/persistence/` contains only `napi_bindings.rs`.

**Exit criterion:** Every check passes. The port is complete.

---

## Concrete child cards (the work to schedule)

Each card below is one piece of the roadmap above. Sized for ≤13 points each. Cards are ordered by dependency; do them in this order.

| Card | Phase | Title | Pts |
|------|-------|-------|-----|
| RPC-031 | 1.1 | Lift `MessageEnvelope` types into `codelet-core::persistence::message_envelope` | 5 |
| RPC-032 | 1.2 | Lift `MessageStore` + `message_index` into `codelet-core::persistence::messages` | 5 |
| RPC-033 | 1.3 | Lift `SessionStore` manifest + `load_session` + `append_message_with_metadata` into `codelet-core::persistence::manifest` | 5 |
| RPC-034 | 1.4 | Lift `BlobStore` into `codelet-core::persistence::blob` | 3 |
| RPC-035 | 1.5 | Reduce `codelet-napi` persistence to thin `napi_bindings.rs` shims | 3 |
| RPC-036 | 2.1–2.4 | Widen `codelet-rpc-types` with every wire-portable shape (`SessionTokens`, `TokenRestoreState`, `SessionModel`, `CompactionProgress`, `CompactionResult`, `WorkUnitContext`, `ThinkingConfig`, pause/HITL types, new `StreamChunk` variants, `FspecResult`, `IsolatedSessionInfo`) | 8 |
| RPC-037 | 3.1–3.5 | Widen `SessionManagerHandle` + `FspecService` + both backends + stub, with cross-transport parity tests | 13 |
| RPC-038 | 4.1 | Create `codelet-sessions` crate skeleton | 2 |
| RPC-039 | 4.2 | Move `BackgroundSession` from `codelet-napi` into `codelet-sessions`, replace NAPI references | 8 |
| RPC-040 | 4.3 | Move `SessionManager` from `codelet-napi` into `codelet-sessions` | 8 |
| RPC-041 | 4.4 | Replace `GLOBAL_CHUNK_CALLBACK` with `tokio::broadcast` sender; rewire NAPI ThreadsafeFunction as a subscriber | 5 |
| RPC-042 | 4.5 | Implement `SessionManagerHandle` for the extracted `SessionManager` | 5 |
| RPC-043 | 4.6–4.7 | Reduce `codelet-napi` to thin adapter (`session_bindings.rs`); update Cargo.toml | 5 |
| RPC-044 | 5.1–5.3 | Wire `codelet-sessions::SessionManager` into `codelet-fspec::common::build_service`; add dependency-rule regression tests | 3 |
| RPC-045 | 6.1–6.3 | AgentView: subscribe to chunks + status broadcasts, handle every new `StreamChunk` variant | 5 |
| RPC-046 | 6.4 (`/clear`) | `/clear` slash command end-to-end | 2 |
| RPC-047 | 6.4 (`/compact`) | `/compact` slash command + compaction progress footer | 5 |
| RPC-048 | 6.4 (`/thinking` inline) | `/thinking off\|low\|med\|high` inline-arg parsing | 1 |
| RPC-049 | 6.4 (`/resume`) | `/resume` durable restore via `restore_session_messages` + `restore_session_token_state` | 5 |
| RPC-050 | 6.4 (`/detach`) + 6.6 | Work-unit context binding (BoardView attach path + SessionHeader chip + `/detach` slash command) | 5 |
| RPC-051 | 6.5 | Keyboard shortcut parity (`Shift+↑/↓` history, `Ctrl+R` search, `Esc` interrupt cascade) | 3 |
| RPC-052 | 6.7 | Pending-input draft persistence on session switch | 2 |
| RPC-053 | 6.8 | Pause / HITL UI (`ConfirmDialog` + `HitlDialog` end-to-end) | 8 |
| RPC-054 | 7.1 | `/provider` ProviderSettingsView + provider-credentials RPC surface | 8 |
| RPC-055 | 7.2 | `/debug` debug-capture wiring | 3 |
| RPC-056 | 7.3 | `/blocklist` view + blocklist RPC surface | 5 |
| RPC-057 | 7.4 | `/merge-worktree` flow + worktree RPC surface | 5 |
| RPC-058 | 7.5 | Lift scheduler engine into `codelet-core::scheduler`; `/schedule` subcommand handler | 8 |
| RPC-059 | 7.6 | Lift loop store into `codelet-core::loops`; `/loop` subcommand handler | 5 |
| RPC-060 | 7.7 | Isolated session creation (`backend.create_isolated_session` + AgentView `/new isolated` flow) | 5 |
| RPC-061 | 7.8 | Supervisor / subordinate links surface | 8 |
| RPC-062 | 7.9 | MCP injection plumbing in extracted `SessionManager` | 3 |
| RPC-063 | 6.4 (`/role`) | `/role` slash command end-to-end (note: trait already wired; just need the UI dialog) | 2 |
| RPC-064 | 6.4 (`/search`) | `/search` slash command end-to-end (note: trait already wired; just need the UI view) | 2 |
| RPC-065 | 8.1 | Behaviour-parity test suite for every slash command + keyboard shortcut | 8 |
| RPC-066 | 8.2 | Cross-frontend integration test against stub provider | 5 |
| RPC-067 | 8.3 | Dependency-rule regression tests for `fspec`, `fspec-tui`, `sessions` | 2 |
| RPC-068 | 8.4–8.5 | Final TS-frontend regression + boundary audit | 3 |

**Total: ~169 points.** Deliverable: a 100% feature-parity Rust AgentView that calls into a NAPI-free `codelet-sessions` crate, with `codelet-napi` reduced to a thin adapter.

---

## Done definition for RPC-030 (this card)

This card is the roadmap. It is done when:

- All child cards RPC-031..RPC-068 above are created in fspec with their parent set to RPC-030 and dependencies wired (each card depends on the previous one in roadmap order unless explicitly parallel).
- The first card (RPC-031) is moved to backlog and prioritised at top of the rust-frontend epic.
