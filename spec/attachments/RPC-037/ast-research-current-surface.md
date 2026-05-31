# AST research: current trait/service surface inventory before RPC-037 widening

Captured via `AstGrep` against the workspace state at the start of RPC-037.

## SessionManagerHandle current trait methods (codelet/core/src/session_manager_handle.rs)

Defaulted (compile unchanged for any handle):
- `get_model_info(&self, session_id) -> ModelInfo`
- `get_thinking_level(&self, session_id) -> ThinkingLevel`
- `list_providers(&self) -> Vec<ProviderInfo>`
- `set_model(&self, session_id, provider_id, model_id) -> Result<(), String>`
- `set_thinking_level(&self, session_id, level) -> Result<(), String>`
- `set_thinking_level_default(&self, session_id, level) -> Result<(), String>`
- `get_role(&self, session_id) -> Option<String>`
- `set_role(&self, session_id, role) -> Result<(), String>`

Required (no default) — caller must implement:
- `list_sessions() -> Vec<SessionInfo>`
- `create_session(role) -> SessionId`
- `send_input(session_id, text)`
- `interrupt(session_id)`
- `get_session_status(session_id) -> SessionStatus`
- `chunks_rx() -> broadcast::Receiver<(SessionId, StreamChunk)>`
- `logs_rx() -> broadcast::Receiver<LogRecord>`
- `chunks_tx() -> broadcast::Sender<(SessionId, StreamChunk)>`
- `logs_tx() -> broadcast::Sender<LogRecord>`

## FspecService current tarpc methods (codelet/rpc/src/lib.rs)

23 methods today:
- `list_work_units`, `list_sessions`, `create_session`, `send_input`, `interrupt`, `get_session_status`, `health`, `checkpoint_counts`, `move_work_unit_up/down`, `get_model_info`, `get_thinking_level`, `get_workspace_info`, `search_files`, `persistence_add_history`, `persistence_get_history`, `persistence_search_history`, `persistence_delete_session`, `list_providers`, `set_session_model`, `set_thinking_level`, `get_session_role`, `set_session_role`

Gap noted in attachment: `set_thinking_level_default` exists on the trait + backend but NOT on the tarpc surface — close in this card.

## FspecBackend trait current methods (codelet/fspec-tui/src/transport/mod.rs)

27 methods today (1:1 with FspecService + the three push-channel subscribe helpers `work_units_rx`, `chunks_rx`, `logs_rx`, plus the no-op default `set_thinking_level_default` and `request_manual_reconnect`).

## RPC-036 wire-portable shapes already lifted into codelet-rpc-types

Confirmed by reading codelet/rpc-types/src/lib.rs (the round-trip test module exercises every shape):
- `SessionTokens`, `TokenRestoreState`, `SessionModel`, `WorkUnitContext`, `ThinkingConfig`
- `PauseKind`, `PauseState`, `PauseResponse`, `ApprovalChoice`
- `HitlOption`, `HitlRequest`, `HitlResponse`
- `IsolatedSessionInfo`
- `StreamChunk::IsolationStateChange` carries `base_commit: Option<String>`

`CompactionResult` (with `compression_ratio`, etc.) already existed in rpc-types pre-card.
`FspecResult` retains its `data: String` byte-compatible shape.

## Methods to add per the RPC-037 attachment

~26 new methods across SessionManagerHandle (each mirrored on FspecService, FspecBackend, both backends, and the stub):

1. `send_input_with_thinking(session_id, text, thinking: Option<ThinkingConfig>)`
2. `get_session_tokens(session_id) -> SessionTokens`
3. `get_session_model(session_id) -> SessionModel`
4. `get_compaction_progress(session_id) -> Option<CompactionProgress>`
5. `get_buffered_output(session_id, limit) -> Vec<StreamChunk>`
6. `clear_history(session_id) -> Result<(), String>`
7. `compact_session(session_id) -> Result<CompactionResult, String>`
8. `restore_session_messages(session_id, envelopes: Vec<String>) -> Result<(), String>`
9. `restore_session_token_state(session_id, state: TokenRestoreState) -> Result<(), String>`
10. `get_work_unit_context(session_id) -> Option<WorkUnitContext>`
11. `set_work_unit_context(session_id, ctx: Option<WorkUnitContext>) -> Result<(), String>`
12. `get_pending_input(session_id) -> Option<String>`
13. `set_pending_input(session_id, text: Option<String>)`
14. `set_active_session(session_id)`
15. `clear_active_session()`
16. `get_active_session() -> Option<SessionId>`
17. `get_effective_cwd(session_id) -> PathBuf`
18. `get_supervisors(session_id) -> Vec<SessionId>`
19. `get_debug_enabled(session_id) -> bool`
20. `set_debug_enabled(session_id, enabled: bool)`
21. `toggle_debug(session_id, debug_dir: &str) -> Result<String, String>`
22. `pause_resume(session_id) -> Result<(), String>`
23. `pause_confirm(session_id, accept: bool) -> Result<(), String>`
24. `pause_triple(session_id, choice: ApprovalChoice) -> Result<(), String>`
25. `send_hitl_response(session_id, response: HitlResponse) -> Result<(), String>`
26. `get_pause_state(session_id) -> Option<PauseState>`
27. `get_hitl_request(session_id) -> Option<HitlRequest>`
28. `send_fspec_result(session_id, result: FspecResult) -> Result<(), String>`
29. `create_isolated_session(role: Option<String>) -> Result<IsolatedSessionInfo, String>`
30. `status_changes_rx() -> broadcast::Receiver<(SessionId, SessionStatus)>`
31. `status_changes_tx() -> broadcast::Sender<(SessionId, SessionStatus)>`
32. `destroy_session(session_id) -> Result<(), String>`

Plus close the gap: add `set_thinking_level_default` to the tarpc FspecService trait (already on SessionManagerHandle + FspecBackend).

## Defaulting policy (per architecture note 0)

Every new trait method declares a default body returning safe sentinels so existing handles compile unchanged:
- collections → empty `Vec::new()`
- options → `None`
- result types → `Ok(())`
- value types → `T::default()`
- `status_changes_rx` default → a fresh broadcast channel whose sender is dropped immediately (subscriber observes `RecvError::Closed` on first poll) — degenerate but type-safe; the stub overrides with a real channel
- `status_changes_tx` default → same — fresh channel sender that is never bound to a real subscriber

## Stub override strategy

`StubSessionManagerHandle` keeps its existing `Arc<Mutex<…>>`-backed shape and adds:
- `Arc<Mutex<HashMap<SessionId, SessionTokens>>>` — keyed seed for `get_session_tokens`
- `Arc<Mutex<HashMap<SessionId, SessionModel>>>` for `get_session_model`
- `Arc<Mutex<HashMap<SessionId, WorkUnitContext>>>` for work-unit binding
- `Arc<Mutex<HashMap<SessionId, String>>>` for pending input
- `Arc<Mutex<Option<SessionId>>>` for active session
- `Arc<Mutex<HashMap<SessionId, bool>>>` for debug
- `Arc<Mutex<HashMap<SessionId, PauseState>>>` for pause state
- `Arc<Mutex<HashMap<SessionId, HitlRequest>>>` for HITL requests
- `status_changes_tx: broadcast::Sender<(SessionId, SessionStatus)>` — the new push-driven channel

Internal helpers seed sensible defaults (compression_ratio 0.5 for compact_session, "/tmp/stub-wt-N" + "abc1234" base_commit for create_isolated_session).

## Cross-transport parity test shape

Reuse the same fixture used by `cross_transport_chunk_parity.rs`:
- Construct a shared StubSessionManagerHandle.
- Construct a SharedFspecService.
- Construct EmbeddedTransport (via EmbeddedFspecBackend) AND bind a ws server (FspecWsClient via WebSocketFspecBackend).
- Run the new-method scenario through each. Assert byte-equal (modulo timestamps / generated IDs) results.

## Files touched (estimate)

- `codelet/core/src/session_manager_handle.rs` — +~700 LoC (trait additions + stub overrides + internal state)
- `codelet/rpc/src/lib.rs` — +~600 LoC (FspecService trait + impls)
- `codelet/fspec-tui/src/transport/mod.rs` — +~400 LoC (FspecBackend additions, with defaults where useful)
- `codelet/fspec-tui/src/transport/embedded.rs` — +~400 LoC of one-line delegates
- `codelet/fspec-tui/src/transport/websocket.rs` — +~600 LoC of guarded delegates
- New: `codelet/rpc-embedded/tests/embedded_widened_handle_parity.rs`
- New: `codelet/rpc-server/tests/cross_transport_widened_parity.rs`

Total ≈ 13 points consistent with the attachment estimate.
