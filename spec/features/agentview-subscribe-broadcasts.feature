@done
@session-management
@rust
@multi-session
@rpc
@agent-view
@tui
@RPC-045
Feature: AgentView: subscribe to chunks + status broadcasts; handle every new StreamChunk variant
  """
  Phase 6.1-6.3 of RPC-030 roadmap. The previous slice RPC-044 wired codelet-sessions::SessionManager into codelet-fspec::build_service so chunks_tx and status_changes_tx broadcasts now have a real source; this slice is the AgentView-side consumer. Subsequent slices (RPC-046+) layer slash commands and dialogs on top of the per-session state introduced here.
  The existing chunks subscriber in rust/fspec-tui/src/app/bootstrap.rs already filters by active_session_tx watch — RPC-045 removes that filter so background sessions accumulate state. The active_session_tx channel itself stays (it is still used by RPC-026 attach paths to seed mode views), only the filter inside the subscriber loop is dropped.
  FspecBackend.status_changes_rx already exists with a closed-receiver default impl (transport/mod.rs line 467). EmbeddedFspecBackend and WebSocketFspecBackend override it to forward the SharedFspecService channel (verified by RPC-037 cross-transport parity tests). RPC-045 just consumes the existing subscription in App::run subscribers.
  IsolationState is a new in-store struct { is_isolated: bool, worktree_path: Option<String>, base_commit: Option<String> } mirroring the StreamChunk::IsolationStateChange wire shape. It lives in store/agent_view/chrome_state.rs (or a new isolation_state.rs sibling) to keep agent_view.rs under its 300-LoC ceiling per the RPC-024 source-shape invariant.
  App::dispatch additions for the new chunk-variant branches live in a new app/dispatch_stream_chunks.rs file (following the dispatch_model_thinking_dialogs/024/025/026 pattern) so app/dispatch.rs stays under the 300-LoC ceiling pinned by rpc024-source-shape.feature. A new try_dispatch_stream_chunks helper is invoked from the existing ChunkReceived arm AFTER the per-session record_chunk + token-state apply but BEFORE the navigator/compositor fanout.
  The FspecCommandRequest runner in this slice is intentionally minimal — happy path covers list-work-units (delegates to backend.list_work_units → JSON-serialise) and show-work-unit (filters the result by the args_json `id` field). Every other command returns an `unsupported command` FspecResult. Wiring a full Rust command dispatcher is out of scope (deferred to a later RPC card once the TS-equivalent surface is also lifted out of NAPI).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. App::run subscribes to backend.chunks_rx() AND backend.status_changes_rx() so chunks and SessionStatus transitions are push-driven on a single tokio task
  #   2. The chunks subscriber no longer filters by the currently-focused session — every (SessionId, StreamChunk) is dispatched as Action::ChunkReceived so background sessions accumulate scrollback and per-session state
  #   3. A new Action::SessionStatusChanged(SessionId, SessionStatus) variant exists and is dispatched whenever the status_changes_rx subscriber receives a (SessionId, SessionStatus) broadcast
  #   4. App::dispatch handling of Action::ChunkReceived also branches on StreamChunk variant and updates new per-session store state: SessionStateChange → session_status, IsolationStateChange → isolation_state, DebugStateChange → debug_enabled, FooterStateUpdate → workspace, FspecCommandRequest → spawn fspec command runner
  #   5. AgentViewStore gains per-session HashMaps for session_status, isolation_state, and debug_enabled, plus accessor methods to read and write them; the existing single-slot workspace field is reused for FooterStateUpdate
  #   6. The FspecCommandRequest runner spawns a tokio::spawn task that executes the requested command (happy path: list-work-units, show-work-unit) and calls backend.send_fspec_result(session_id, FspecResult) without blocking the App task; unsupported commands return FspecResult { success: false, error: Some("unsupported command: <name>"), .. }
  #   7. No polling of get_session_status anywhere in fspec-tui — SessionFooter and any status-pill rendering reads agent_view_store.session_status_for(&id) which is push-updated by the broadcast subscriber
  #   8. Broadcast lag on chunks_rx and status_changes_rx is logged via tracing::warn but does NOT crash the App or close the subscriber loops
  #
  # EXAMPLES:
  #   1. Given two open sessions s-1 (focused) and s-2 (background), when chunks_rx broadcasts (s-2, StreamChunk::text("hi")), then s-2's SessionContext scrollback gains the chunk while s-1's scrollback is untouched
  #   2. Given a session emits StreamChunk::SessionStateChange { state: SessionState::Running }, when App::dispatch handles the resulting Action::ChunkReceived, then agent_view_store.session_status_for(&id) returns SessionStatus::Running
  #   3. Given a session emits StreamChunk::IsolationStateChange { is_isolated: true, worktree_path: Some("/tmp/wt"), base_commit: Some("abc123") }, when App::dispatch handles it, then agent_view_store.isolation_state_for(&id) returns Some(IsolationState { is_isolated: true, worktree_path: Some("/tmp/wt"), base_commit: Some("abc123") })
  #   4. Given a session emits StreamChunk::DebugStateChange { enabled: true }, when App::dispatch handles it, then agent_view_store.debug_enabled_for(&id) returns true
  #   5. Given a session emits StreamChunk::FooterStateUpdate { cwd: "/Users/alice/proj", display_path: "~/proj", is_git_repo: true, branch: Some("main") }, when App::dispatch handles it, then agent_view_store.workspace() returns a WorkspaceInfo whose cwd equals "/Users/alice/proj" and git_branch equals Some("main")
  #   6. Given a session emits StreamChunk::FspecCommandRequest { fspec_request: FspecRequest { command: "list-work-units", args_json: "{}", project_root: <tempdir>, tool_call_id: "t-1" } }, when App::dispatch handles it, then within 1 second backend.send_fspec_result is called with FspecResult { success: true, tool_call_id: "t-1", data: <JSON array of work units>, .. }
  #   7. Given a session emits StreamChunk::FspecCommandRequest with command "unknown-command", when App::dispatch handles it, then backend.send_fspec_result is called with FspecResult { success: false, error: Some("unsupported command: unknown-command"), tool_call_id: <unchanged> }
  #   8. Given the status_changes_rx broadcasts (s-1, SessionStatus::Running), when the status subscriber forwards Action::SessionStatusChanged(s-1, Running) to App::dispatch, then agent_view_store.session_status_for(&s-1) returns SessionStatus::Running
  #   9. Given a backend whose chunks_rx Sender drops abruptly (RecvError::Closed), when the chunks subscriber observes the close, then the subscriber loop exits cleanly without panicking
  #
  # ========================================
  Background: User Story
    As a user with multiple open AgentView sessions
    I want to see live session status, isolation, debug, footer, and fspec-command updates push-broadcast from the backend without polling
    So that every per-session UI element stays in sync with the SessionManager in real time even for background sessions

  Scenario: Background-session chunks are routed by SessionId, not by focus
    Given an App with two open sessions s-1 (focused) and s-2 (background)
    When the chunks subscriber forwards Action::ChunkReceived(s-2, StreamChunk::text("hi"))
    Then the App's dispatch routes the chunk into s-2's SessionContext scrollback
    And s-1's SessionContext scrollback remains empty

  Scenario: SessionStateChange chunk updates per-session status in the store
    Given an App with an open session s-1
    When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::SessionStateChange { state: SessionState::Running })
    Then agent_view_store.session_status_for(&s-1) returns SessionStatus::Running

  Scenario: IsolationStateChange chunk updates per-session isolation state
    Given an App with an open session s-1
    When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::IsolationStateChange { is_isolated: true, worktree_path: Some("/tmp/wt"), base_commit: Some("abc123") })
    Then agent_view_store.isolation_state_for(&s-1) returns an IsolationState whose is_isolated is true
    And the stored IsolationState worktree_path equals Some("/tmp/wt")
    And the stored IsolationState base_commit equals Some("abc123")

  Scenario: DebugStateChange chunk updates per-session debug flag
    Given an App with an open session s-1
    When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::DebugStateChange { enabled: true })
    Then agent_view_store.debug_enabled_for(&s-1) returns true

  Scenario: FooterStateUpdate chunk refreshes the shared workspace info
    Given an App with an open session s-1
    When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::FooterStateUpdate { cwd: "/Users/alice/proj", display_path: "~/proj", is_git_repo: true, branch: Some("main") })
    Then agent_view_store.workspace() returns Some(WorkspaceInfo)
    And the stored WorkspaceInfo.cwd equals "/Users/alice/proj"
    And the stored WorkspaceInfo.git_branch equals Some("main")

  Scenario: FspecCommandRequest for list-work-units round-trips back via send_fspec_result
    Given an App with an open session s-1 wired to a MockBackend that has seeded work units
    When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::FspecCommandRequest { fspec_request: FspecRequest { command: "list-work-units", args_json: "{}", project_root: <tempdir>, tool_call_id: "t-1" } })
    Then within 1 second backend.send_fspec_result is called exactly once
    And the captured FspecResult has success == true and tool_call_id == "t-1"
    And the captured FspecResult.data is a JSON-serialised array containing every seeded work unit

  Scenario: FspecCommandRequest with an unsupported command returns an error result
    Given an App with an open session s-1
    When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::FspecCommandRequest { fspec_request: FspecRequest { command: "unknown-command", args_json: "{}", project_root: ".", tool_call_id: "t-2" } })
    Then within 1 second backend.send_fspec_result is called exactly once
    And the captured FspecResult has success == false
    And the captured FspecResult.error equals Some("unsupported command: unknown-command")
    And the captured FspecResult.tool_call_id equals "t-2"

  Scenario: SessionStatusChanged Action updates per-session status push-driven from status_changes_rx
    Given an App with an open session s-1
    When the status subscriber forwards Action::SessionStatusChanged(s-1, SessionStatus::Running)
    Then agent_view_store.session_status_for(&s-1) returns SessionStatus::Running

  Scenario: chunks_rx Sender drop terminates the subscriber loop without panicking
    Given an App whose chunks_rx Sender is dropped before any chunk has been broadcast
    When the chunks subscriber task is awaited
    Then the subscriber task completes cleanly without panicking
