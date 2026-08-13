@RPC-420
@work-management
@codelet
@done
@schema-design
@session-management
@rpc
@rust
@RPC-037
Feature: Widen SessionManagerHandle + FspecService + both backends + stub with cross-transport parity tests
  """
  Architecture: send_input on SessionManagerHandle keeps its existing 2-arg signature (session_id, text). A new send_input_with_thinking is added with a default that calls send_input. The rust/napi SessionManager implementation gets to override either or both; current SessionManagerHandle::send_input default in StubSessionManagerHandle keeps working.
  Architecture: status_changes broadcast capacity is 256 (same as the existing chunks broadcast in the stub via with_capacity defaults). The default trait impl returns a fresh local channel whose sender is dropped immediately so polling get_session_status remains the fallback path for any handle that hasn't wired push status.
  Architecture: the FspecBackend trait surface in rust/fspec-tui/src/transport/mod.rs grows ~30 methods. Each is a one-line tarpc client delegate on EmbeddedFspecBackend; on WebSocketFspecBackend each follows the existing `let guard = self.client.read().await; let client = guard.as_ref().ok_or(BackendError::Disconnected)?; ... client.client().<rpc>(context::current(), args).await?` pattern. async-trait already in scope; no new dependencies.
  Architecture: the FspecBackend trait gains a status_changes_rx() -> broadcast::Receiver<(SessionId, SessionStatus)> method paired with chunks_rx and logs_rx. For the WebSocket backend a new internal broadcast channel is fed by a new Envelope::StatusUpdate variant pushed by the server fanout task — analogous to how chunks/logs/work-units are fanned today. This card adds the variant and the pump/fanout wiring.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every new method declared on the SessionManagerHandle trait MUST carry a default body so existing handles (StubSessionManagerHandle pre-override, any future NAPI/test handles) compile unchanged — defaults return safe sentinels: empty collections, None, Ok(()), SessionTokens::default(), SessionModel::default()-style instances, etc.
  #   2. send_input on SessionManagerHandle keeps its existing 2-arg signature for backward compatibility; a sibling send_input_with_thinking(session_id, text, thinking: Option<ThinkingConfig>) is added. The existing send_input default delegates to send_input_with_thinking(.., None) so implementers only override one method.
  #   3. Every trait method added to SessionManagerHandle MUST have a peer async method on FspecService (rust/rpc/src/lib.rs). FspecServiceImpl delegates each call through self.inner.session_manager() returning the same safe defaults when no handle is attached. The pre-existing set_thinking_level_default gap on the tarpc trait is closed in this card.
  #   4. Every new tarpc method is mirrored on the FspecBackend trait (rust/fspec-tui/src/transport/mod.rs) and implemented on BOTH EmbeddedFspecBackend (one-line delegate through self.client) and WebSocketFspecBackend (read-lock the client slot, fail with BackendError::Disconnected when None, otherwise one-line delegate).
  #   5. StubSessionManagerHandle MUST override every new trait method with a deterministic, idempotent stub: get_session_tokens returns seeded SessionTokens, clear_history emits a SessionStateChange { state: Cleared } chunk + Ok(()) (RPC-074: was UserNotification, retired for TS parity), compact_session returns canned CompactionResult, pause_* / send_hitl_response set internal flags + emit SessionStateChange, status_changes_tx/rx expose a new internal broadcast::Sender<(SessionId, SessionStatus)>. The stub MUST be deterministic so cross-transport parity tests produce identical output across both transports.
  #   6. A push-driven status_changes broadcast channel of (SessionId, SessionStatus) is introduced on SessionManagerHandle. status_changes_rx() returns a broadcast::Receiver; status_changes_tx() exposes the Sender for the host/co-listener. The Stub owns a real channel; default implementations on the trait return a fresh local channel whose sender is dropped immediately so subscribers observe Closed (safe degenerate behaviour for handles that haven't wired status push yet).
  #   7. Cross-transport parity tests live in rust/rpc-embedded/tests/ and rust/rpc-server/tests/. A scenario runs the same call sequence through EmbeddedFspecBackend AND WebSocketFspecBackend constructed against the same StubSessionManagerHandle, and asserts every method's return value is byte-identical (modulo timestamps and any source-of-truth ID minted inside the stub).
  #   8. No existing test in rust/rpc-embedded/tests/ or rust/rpc-server/tests/ is broken by the additions — every new method has a default impl on the trait so any pre-existing call site keeps compiling. cargo build of codelet-core, codelet-rpc, codelet-rpc-types, codelet-rpc-embedded, codelet-rpc-server, codelet-fspec-tui all pass. cargo clippy -p codelet-core -- -D warnings is clean.
  #
  # EXAMPLES:
  #   1. send_input keeps the existing trait shape (session_id, text). A NEW method send_input_with_thinking(session_id, text, thinking: Option<ThinkingConfig>) gets a new peer FspecService::send_input_with_thinking; the trait's send_input default forwards to send_input_with_thinking(.., None). Backend trait gains send_input_with_thinking that defaults to send_input when None thinking is supplied so the WS backend keeps the same wire shape.
  #   2. Engineer calls backend.compact_session(sid).await on either transport; with the deterministic stub the return is Ok(CompactionResult { compression_ratio: 50.0, original_tokens: 1000, compacted_tokens: 500, turns_summarized: 4, turns_kept: 2 }) and a StreamChunk::CompactionComplete arrives on chunks_rx for that session
  #   3. Engineer subscribes to backend.status_changes_rx() on either transport; calls backend.send_input(sid, "hi") and then receives (sid, SessionStatus::Running) followed by (sid, SessionStatus::Idle) within 5 seconds — push-driven, no polling get_session_status
  #   4. Engineer calls backend.set_pending_input(sid, Some("draft text")) then backend.get_pending_input(sid).await returns Ok(Some("draft text")). On both transports the round-trip works identically against the same StubSessionManagerHandle
  #   5. Engineer calls backend.create_isolated_session(Some("reviewer".to_string())).await on either transport; the stub returns Ok(IsolatedSessionInfo { session_id: SessionId::new("stub-iso-1"), worktree_path: "/tmp/stub-wt-1".into(), base_commit: "abc1234".into() }) and the subsequent list_sessions includes the newly-minted session
  #   6. Engineer calls backend.pause_confirm(sid, true).await; subsequently backend.get_pause_state(sid) returns None (the pause was resolved) and a StreamChunk::SessionStateChange arrives on chunks_rx with state Running. Same for pause_triple(sid, ApprovalChoice::Approve) and pause_resume(sid).
  #   7. Engineer calls backend.destroy_session(sid).await on either transport. After Ok(()) returns, backend.list_sessions().await no longer contains sid
  #   8. Engineer wires a SharedFspecService against StubSessionManagerHandle, constructs EmbeddedFspecBackend + WebSocketFspecBackend, runs the same scenario (create session → set_pending_input → send_input → drain chunks → compact_session → destroy_session) through each, and asserts the resulting Vec<StreamChunk> is byte-equal modulo timestamps under bincode::serialize
  #
  # ========================================
  Background: User Story
    As a fspec backend engineer
    I want to extend SessionManagerHandle, FspecService, both FspecBackend transports, and the deterministic stub with every additional method the Rust AgentView needs to drive a session through either embedded or WebSocket transport (send_input_with_thinking, get_session_tokens/model, compaction progress + ops, history clear/restore, work-unit context, pending input, active session tracking, effective cwd, supervisors, debug capture, pause/HITL, fspec round-trip, isolated session create, status_changes broadcast, destroy_session, set_thinking_level_default on the RPC trait)
    So that after this card the Rust AgentView can drive any TS-equivalent session action through the FspecBackend trait under either transport, with cross-transport parity tests proving the embedded and WebSocket paths emit byte-identical results when run against the deterministic stub

  Scenario: send_input_with_thinking is added as a sibling of send_input with backward-compatible default forwarding
    Given an engineer holds an Arc<dyn SessionManagerHandle> backed by StubSessionManagerHandle
    When the engineer calls handle.send_input_with_thinking(&sid, "hi".to_string(), None)
    Then the call returns immediately
    And subsequent chunks_rx().recv() yields a StreamChunk::Text { text: "hi back", .. } for that session
    And calling handle.send_input(&sid, "hi".to_string()) (the existing 2-arg shape) produces the exact same chunk sequence
    And the SessionManagerHandle trait declares fn send_input_with_thinking with a default body that delegates to self.send_input(sid, text) when thinking is None
    And FspecService::send_input_with_thinking exists with the same arg list as the trait method modulo Context

  Scenario: get_session_tokens / get_session_model / get_compaction_progress return safe defaults via the stub
    Given an engineer holds a StubSessionManagerHandle that has not been seeded with custom token state
    When the engineer calls backend.get_session_tokens(sid).await over the embedded transport
    Then the call returns Ok(SessionTokens { input_tokens: 0, output_tokens: 0 })
    When the engineer calls backend.get_session_model(sid).await
    Then the call returns Ok(SessionModel { provider_id: "", model_id: "", context_window: 0, max_output_tokens: 0, compaction_threshold: 0 })
    When the engineer calls backend.get_compaction_progress(sid).await
    Then the call returns Ok(None)
    And the same three calls over WebSocketFspecBackend against a server hosting the SAME StubSessionManagerHandle return identical values

  Scenario: clear_history emits a SessionStateChange chunk and returns Ok
    Given an engineer subscribes to backend.chunks_rx() before calling clear_history
    When the engineer calls backend.clear_history(sid).await on either transport
    Then the call returns Ok(())
    And within 1 second a StreamChunk::SessionStateChange chunk with state SessionState::Cleared for that session is observed on chunks_rx (RPC-074: TS parity with TUI-066 contract; previously this was a UserNotification chunk, retired as a Rust-side invention)

  Scenario: compact_session returns the canned CompactionResult and emits CompactionComplete
    Given an engineer subscribes to backend.chunks_rx() before calling compact_session
    When the engineer calls backend.compact_session(sid).await on either transport
    Then the call returns Ok(CompactionResult { compression_ratio: 50.0, original_tokens: 1000, compacted_tokens: 500, turns_summarized: 4, turns_kept: 2 })
    And within 1 second a StreamChunk::CompactionComplete arrives on chunks_rx for that session carrying the same CompactionResult

  Scenario: restore_session_messages and restore_session_token_state are wired through both transports
    Given an engineer holds a freshly-created session id from backend.create_session(None).await
    When the engineer calls backend.restore_session_messages(sid, vec!["{}".to_string()]).await on either transport
    Then the call returns Ok(())
    When the engineer calls backend.restore_session_token_state(sid, TokenRestoreState { current_context: 1, cumulative_billed_output: 2, cache_read: 3, cache_creation: 4, cumulative_billed_input: 5, cumulative_billed_output_second: 6 }).await
    Then the call returns Ok(())

  Scenario: work-unit context get/set round-trips through both transports
    Given an engineer holds a freshly-created session id
    When the engineer calls backend.set_work_unit_context(sid, Some(WorkUnitContext { id: "AUTH-001".into(), title: "Login".into(), status: "implementing".into() })).await
    Then the call returns Ok(())
    When the engineer calls backend.get_work_unit_context(sid).await
    Then the call returns Ok(Some(WorkUnitContext { id: "AUTH-001", title: "Login", status: "implementing" }))
    When the engineer calls backend.set_work_unit_context(sid, None).await followed by backend.get_work_unit_context(sid).await
    Then the second get call returns Ok(None)

  Scenario: pending_input draft text round-trips through both transports
    Given an engineer holds a freshly-created session id
    When the engineer calls backend.set_pending_input(sid, Some("draft text".to_string())).await on either transport
    Then the call returns Ok(())
    When the engineer calls backend.get_pending_input(sid).await
    Then the call returns Ok(Some("draft text".to_string()))
    When the engineer calls backend.set_pending_input(sid, None).await followed by backend.get_pending_input(sid).await
    Then the second get call returns Ok(None)

  Scenario: active session tracking get/set/clear round-trips
    Given an engineer holds two distinct session ids minted via create_session
    When the engineer calls backend.set_active_session(sid_a).await
    Then backend.get_active_session().await returns Ok(Some(sid_a))
    When the engineer calls backend.set_active_session(sid_b).await
    Then backend.get_active_session().await returns Ok(Some(sid_b))
    When the engineer calls backend.clear_active_session().await
    Then backend.get_active_session().await returns Ok(None)

  Scenario: get_effective_cwd / get_supervisors return safe defaults via the stub on both transports
    Given an engineer holds a freshly-created session id
    When the engineer calls backend.get_effective_cwd(sid).await
    Then the call returns Ok(PathBuf::from("")) (the stub default — an empty PathBuf)
    When the engineer calls backend.get_supervisors(sid).await
    Then the call returns Ok(Vec::new())

  Scenario: debug capture toggle is wired through both transports
    Given an engineer holds a freshly-created session id and subscribes to chunks_rx
    When the engineer calls backend.get_debug_enabled(sid).await
    Then the call returns Ok(false)
    When the engineer calls backend.set_debug_enabled(sid, true).await
    Then the call returns Ok(())
    When the engineer calls backend.get_debug_enabled(sid).await
    Then the call returns Ok(true)
    When the engineer calls backend.toggle_debug(sid, "/tmp/debug").await
    Then the call returns Ok(<some path string>) and a StreamChunk::DebugStateChange chunk is observed on chunks_rx for that session

  Scenario: pause_confirm / pause_triple / pause_resume update pause state and emit SessionStateChange
    Given an engineer seeds the StubSessionManagerHandle with a PauseState { kind: PauseKind::Confirm, prompt: "Apply?", tool_call_id: None } for sid and subscribes to chunks_rx
    When the engineer calls backend.get_pause_state(sid).await
    Then the call returns Ok(Some(PauseState { kind: PauseKind::Confirm, prompt: "Apply?", tool_call_id: None }))
    When the engineer calls backend.pause_confirm(sid, true).await
    Then the call returns Ok(())
    And a StreamChunk::SessionStateChange { state: SessionState::Running } arrives on chunks_rx for sid within 1 second
    And backend.get_pause_state(sid).await returns Ok(None)
    When the engineer seeds a PauseKind::Triple pause and calls backend.pause_triple(sid, ApprovalChoice::Approve).await
    Then the call returns Ok(()) and backend.get_pause_state(sid).await returns Ok(None)
    When the engineer seeds another pause and calls backend.pause_resume(sid).await
    Then the call returns Ok(()) and backend.get_pause_state(sid).await returns Ok(None)

  Scenario: HITL request/response round-trips through both transports
    Given an engineer seeds the StubSessionManagerHandle with a HitlRequest { id: "q-1", question: "Apply?", header: "Apply", options: [HitlOption{label:"Yes",..}, HitlOption{label:"No",..}], allow_text_input: true } for sid
    When the engineer calls backend.get_hitl_request(sid).await
    Then the call returns Ok(Some(<the seeded HitlRequest>))
    When the engineer calls backend.send_hitl_response(sid, HitlResponse { id: "q-1".into(), value: "Yes".into() }).await
    Then the call returns Ok(())
    And backend.get_hitl_request(sid).await subsequently returns Ok(None)

  Scenario: send_fspec_result round-trips through both transports
    Given an engineer holds a freshly-created session id
    When the engineer calls backend.send_fspec_result(sid, FspecResult { success: true, data: "{}".into(), error: None, system_reminder: None, tool_call_id: "tc-1".into() }).await on either transport
    Then the call returns Ok(())

  Scenario: create_isolated_session returns IsolatedSessionInfo and the session is listed
    Given an engineer holds an EmbeddedFspecBackend backed by the StubSessionManagerHandle
    When the engineer calls backend.create_isolated_session(Some("reviewer".to_string())).await
    Then the call returns Ok(IsolatedSessionInfo { session_id: <minted SessionId>, worktree_path: non-empty String, base_commit: non-empty String })
    And backend.list_sessions().await contains a SessionInfo with id == iso_info.session_id.value and is_isolated == true
    And calling the same on WebSocketFspecBackend against a server hosting the SAME stub produces an IsolatedSessionInfo with the SAME deterministic shape

  Scenario: set_thinking_level_default closes the tarpc-side gap and round-trips through both transports
    Given an engineer holds a freshly-created session id
    When the engineer calls backend.set_thinking_level_default(sid, ThinkingLevel::High).await on either transport
    Then the call returns Ok(())
    And FspecService::set_thinking_level_default exists in rust/rpc/src/lib.rs as a new tarpc method on the service trait

  Scenario: status_changes_rx is push-driven on both transports
    Given an engineer subscribes to backend.status_changes_rx() on either transport before calling send_input
    When the engineer calls backend.send_input(sid, "hi".to_string()).await
    Then within 5 seconds the status_changes_rx receiver yields (sid, SessionStatus::Running)
    And within a further 5 seconds the receiver yields (sid, SessionStatus::Idle)
    And the same (sid, status) tuple sequence is observed on the WebSocket transport when the WS server is hosting the SAME StubSessionManagerHandle

  Scenario: destroy_session removes the session from list_sessions on both transports
    Given an engineer holds a freshly-created session id via backend.create_session(None).await
    And backend.list_sessions().await contains sid
    When the engineer calls backend.destroy_session(sid).await
    Then the call returns Ok(())
    And backend.list_sessions().await no longer contains sid

  Scenario: every new method has a tarpc service peer and a FspecBackend trait peer
    Given the engineer opens rust/core/src/session_manager_handle.rs and rust/rpc/src/lib.rs and rust/fspec-tui/src/transport/mod.rs after this card lands
    Then for every method added by this card to SessionManagerHandle there is an async fn of the same name (modulo Context) on the FspecService tarpc trait
    And for every async fn added to FspecService there is an async fn on the FspecBackend trait
    And EmbeddedFspecBackend implements every new FspecBackend method as a one-line delegate through self.client
    And WebSocketFspecBackend implements every new FspecBackend method using the existing client.read().await + BackendError::Disconnected guard pattern

  Scenario: cross-transport byte-identical parity for the happy-path scenario
    Given a SharedFspecService is constructed against a freshly-built StubSessionManagerHandle
    And both EmbeddedFspecBackend and WebSocketFspecBackend are constructed against that service
    When the engineer runs the same happy-path scenario through each backend: create_session(None) → send_input(sid, "hi") → drain chunks_rx until StreamChunk::Done → destroy_session(sid)
    Then bincode::serialize of the captured Vec<StreamChunk> from the embedded path equals the same from the WebSocket path
    And no existing tarpc / push-channel test in rust/rpc-embedded/tests/ or rust/rpc-server/tests/ regresses

  Scenario: cargo build and clippy stay green
    Given the engineer is at the workspace root /Users/rquast/projects/fspec/codelet
    When the engineer runs `cargo build -p codelet-core -p codelet-rpc -p codelet-rpc-types -p codelet-rpc-embedded -p codelet-rpc-server -p codelet-fspec-tui`
    Then every crate builds without errors
    When the engineer runs `cargo clippy -p codelet-core -- -D warnings`
    Then clippy reports no warnings
