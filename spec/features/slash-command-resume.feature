@done
@slash-command
@tui
@agent-view
@rpc
@multi-session
@rust
@session-management
@RPC-049
Feature: /resume durable restore via restore_session_messages + restore_session_token_state
  """
  Wire shape: new `resume_session` trait method on SessionManagerHandle with a default impl that performs: (1) Uuid::parse_str(session_id), (2) codelet_core::persistence::load_session(uuid), (3) codelet_core::persistence::get_session_message_envelopes(uuid), (4) build TokenRestoreState from manifest.token_usage, (5) self.restore_session_messages(session_id, envelopes), (6) self.restore_session_token_state(session_id, state). Errors at any step propagate as Result<(), String>.
  Lift target: `get_session_message_envelopes(uuid: Uuid) -> Result<Vec<String>, String>` becomes a public free function in `rust/core/src/persistence/manifest.rs` mirroring the existing `get_session_messages` function above it. The implementation ports the body of `persistence_get_session_message_envelopes` from `rust/napi/src/persistence/napi_bindings.rs:729` minus the napi::Error wrapping (use plain String errors). The NAPI binding becomes a one-line delegate `codelet_core::persistence::get_session_message_envelopes(uuid).map_err(Error::from_reason)`.
  Action plumbing: extend the components::Action enum with one new variant: `SessionResumeComplete(SessionId)`. The error path re-uses the existing `EmitSessionNotice(SessionId, String)` variant (RPC-046) so the failure notice arrives via the standard scrollback-routing path.
  Dispatch wiring: extend `app/dispatch_resume_search_views.rs::handle_attach_to_session` to spawn a tokio task that awaits `backend.resume_session(session_id)` and dispatches either `Action::SessionResumeComplete(id)` (Ok) or `Action::EmitSessionNotice(id, format!("[error] /resume failed: {e}"))` (Err). A NEW `handle_session_resume_complete` helper in the same file spawns a second task that calls `backend.get_buffered_output(id, 1000)` and replays each returned chunk as `Action::ChunkReceived(id, chunk)`. App::dispatch gains one match arm `Action::SessionResumeComplete(id) => self.handle_session_resume_complete(id.clone())`. dispatch_resume_search_views.rs stays under 300 LoC because the additions are ~30 LoC.
  RPC surface: `FspecService::resume_session(self, ctx, session_id: SessionId) -> Result<(), String>` is added in `rust/rpc/src/lib.rs`; `FspecServiceImpl::resume_session` delegates through `self.inner.session_manager()?.resume_session(&session_id)`; default-when-no-handle returns Ok(()). Both EmbeddedFspecBackend and WebSocketFspecBackend gain `async fn resume_session(&self, session_id: SessionId) -> Result<()>` as one-line delegates following the same shape as their existing `clear_history` / `compact_session` impls. The StubSessionManagerHandle override increments an AtomicU64 call counter so cross-transport parity tests can assert call counts.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A new `resume_session(session_id) -> Result<(), String>` method MUST exist on SessionManagerHandle (with a default impl that orchestrates load_session + get_session_message_envelopes + restore_session_messages + restore_session_token_state), on FspecService (tarpc), and on the FspecBackend trait — implemented as a one-line delegate on both EmbeddedFspecBackend and WebSocketFspecBackend.
  #   2. `get_session_message_envelopes(session_id: Uuid) -> Result<Vec<String>, String>` MUST be lifted from `rust/napi/src/persistence/napi_bindings.rs` into `codelet_core::persistence::manifest` (or messages) as a public free function; the NAPI binding `persistence_get_session_message_envelopes` becomes a thin one-line delegate that preserves the byte-identical TS surface (synthetic compaction summaries + blob rehydration behaviour included).
  #   3. The ResumeSessionView's `Selected(SessionId)` outcome (already dispatched as `Action::AttachToSession`) MUST be extended so that AFTER the synchronous open_sessions move/append, a tokio task is spawned that awaits `backend.resume_session(session_id)` and routes the outcome via two new actions: `Action::SessionResumeComplete(SessionId)` on success and `Action::EmitSessionNotice(session_id, "[error] /resume failed: {reason}")` on failure.
  #   4. On `Action::SessionResumeComplete(session_id)`, App::dispatch MUST spawn a tokio task that calls `backend.get_buffered_output(session_id, 1000).await` and re-emits each returned chunk into the action bus as `Action::ChunkReceived(session_id, chunk)` — so the resumed session's scrollback is seeded from the backend's replay buffer.
  #   5. When `Action::AttachToSession` fires for a session already present in open_sessions, the existing focus-move semantics from RPC-026 are preserved AND `backend.resume_session` is STILL called (idempotent re-restore is acceptable parity with the TS handleResumeMode). The resumed-chunk re-seed is NOT deduplicated against existing scrollback entries — replay buffer arrival follows the same path as fresh streaming chunks.
  #   6. MockBackend in `rust/fspec-tui/tests/common/mod.rs` MUST gain (a) `resume_session_calls()` counter + `last_resume_session()` accessor, (b) `set_resume_session_error(message)` to script the Err branch, (c) `set_buffered_output(chunks)` to script the get_buffered_output replay set used by SessionResumeComplete, and (d) full `async fn resume_session(&self, id: SessionId) -> Result<()>` impl.
  #   7. Cross-transport parity: `resume_session(id)` round-trips identically across EmbeddedFspecBackend and WebSocketFspecBackend against the SAME StubSessionManagerHandle (the StubSessionManagerHandle override returns Ok(()) deterministically); the RPC-037 cross-transport parity test suite is extended with a scenario asserting parity for the new method.
  #   8. Source-shape regression: no new file under `rust/fspec-tui/src/` exceeds 300 LoC; `dispatch.rs` stays under the 300-LoC ceiling — the new AttachToSession / SessionResumeComplete arms are factored into a new `app/dispatch_rpc049.rs` helper file (or, if minimal, an addition to `dispatch_resume_search_views.rs` that keeps it under 300 LoC). codelet-fspec-tui MUST NOT depend on codelet-napi (RPC-002 invariant).
  #
  # EXAMPLES:
  #   1. Given an App wired to a MockBackend with sessions ['s-1', 's-2'] and ResumeSessionView open with row 0 (s-1) selected, when the user presses Enter, then backend.resume_session(s-1) is called exactly once within 1 second AND open_sessions contains s-1 (append-or-focus semantics from RPC-026 preserved).
  #   2. Given a MockBackend whose resume_session(s-1) returns Ok(()) and whose buffered_output for s-1 is [StreamChunk::text("hello"), StreamChunk::text("world")], when AttachToSession(s-1) is dispatched, then within 1 second the SessionContext for s-1 contains two scrollback chunks whose text is 'hello' and 'world'.
  #   3. Given a MockBackend whose resume_session(s-1) returns Err("corrupt manifest"), when AttachToSession(s-1) is dispatched, then within 1 second the SessionContext for s-1 contains a scrollback line whose text equals '[error] /resume failed: corrupt manifest' AND get_buffered_output is NEVER called.
  #   4. Given a MockBackend with a scripted sequence of envelopes for session s-1 ([{type:'user',text:'hi'},{type:'assistant',text:'hello'}]) loaded into buffered_output, when the user picks s-1 from /resume, then the scrollback (after the full async chain completes) shows BOTH the user and assistant lines in order.
  #   5. Cross-transport parity: build a SharedFspecService with the StubSessionManagerHandle; construct EmbeddedFspecBackend AND WebSocketFspecBackend over the same service; call backend.resume_session(SessionId("stub-1")) through each; assert both return Ok(()) and the StubSessionManagerHandle's internal resume_session call counter increments to 2 (once per transport).
  #   6. codelet_core::persistence::get_session_message_envelopes(uuid) for a non-existent session returns Err("..."); for an existing manifest with N messages returns Ok(Vec<String>) with N JSON envelopes (each parseable via serde_json::from_str into a serde_json::Value with a 'message' field). The lifted free function produces byte-identical output to the previous NAPI binding for the same input manifest.
  #   7. Source-shape regression: a grep over `rust/fspec-tui/src/` finds zero matches for `codelet_napi`; every file under `rust/fspec-tui/src/` (including the new dispatch_rpc049.rs if introduced) is < 300 LoC; `rust/fspec-tui/src/app/dispatch.rs` is < 300 LoC after the new AttachToSession + SessionResumeComplete arms are wired.
  #
  # ========================================
  Background: User Story
    As a fspec user with a stored session on disk
    I want to select that session from the /resume picker
    So that my prior conversation and token state are restored into the active TUI so I can continue the work

  Scenario: AttachToSession spawns backend.resume_session for the selected session
    Given an App wired to a MockBackend with sessions ["s-1", "s-2"]
    And ResumeSessionView is open with row 0 (s-1) selected
    When the user presses Enter on the resume view
    Then within 1 second backend.resume_session is called exactly once with session_id s-1
    And the AgentViewStore's open_sessions contains s-1
    And the AgentViewStore's current_session is s-1

  Scenario: SessionResumeComplete seeds scrollback from get_buffered_output on Ok
    Given an App wired to a MockBackend whose resume_session returns Ok(())
    And the MockBackend's buffered_output for s-1 is [StreamChunk::text("hello"), StreamChunk::text("world")]
    When Action::AttachToSession(s-1) is dispatched
    Then within 1 second backend.resume_session is called exactly once with session_id s-1
    And within 1 second backend.get_buffered_output is called exactly once with session_id s-1 and limit 1000
    And within 1 second the SessionContext for s-1 contains a scrollback chunk whose text equals "hello"
    And within 1 second the SessionContext for s-1 contains a scrollback chunk whose text equals "world"

  Scenario: SessionResumeFailed emits an error notice and skips get_buffered_output
    Given an App wired to a MockBackend whose resume_session returns Err("corrupt manifest")
    When Action::AttachToSession(s-1) is dispatched
    Then within 1 second the SessionContext for s-1 contains a scrollback chunk whose text equals "[error] /resume failed: corrupt manifest"
    And backend.get_buffered_output is NEVER called

  Scenario: Idempotent re-restore — AttachToSession on an already-open session still calls resume_session
    Given an App with open_sessions [s-1] and current_session s-1
    And the MockBackend's resume_session returns Ok(())
    When Action::AttachToSession(s-1) is dispatched
    Then within 1 second backend.resume_session is called exactly once with session_id s-1
    And open_sessions length stays 1 (no duplicate append)
