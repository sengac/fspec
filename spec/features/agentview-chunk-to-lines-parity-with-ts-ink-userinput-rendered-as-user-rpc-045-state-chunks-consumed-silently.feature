@done
@bug-fix
@scrollback
@rust
@tui
@agent-view
@RPC-071
Feature: AgentView chunk_to_lines parity with TS Ink: UserInput rendered as `user>`, RPC-045 state chunks consumed silently
  """
  AgentView chunk renderer is exhaustive (no catch-all Debug arm). Visible variants: UserInput->'user> {text}', Text->'assistant> {text}', Thinking->'(thinking) {text}', IncomingMessage->'supervisor> {text}', UserNotification->'[notice] {message}', Error->'[error] {error}', Interrupted->'[interrupted] {n} queued', Done->'[done]'. Silent variants (return None): SessionStateChange, IsolationStateChange, DebugStateChange, FooterStateUpdate, FspecCommandRequest, FspecCommandResult, WorkUnitsUpdate, SupervisorPendingInjection, CompactionComplete, TokenUpdate, ContextFillUpdate, ToolCall, ToolResult, ToolProgress.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. UserInput chunks render as a `user> {text}` scrollback line
  #   2. SessionStateChange / IsolationStateChange / DebugStateChange / FooterStateUpdate / FspecCommandRequest / FspecCommandResult / WorkUnitsUpdate / SupervisorPendingInjection / CompactionComplete / TokenUpdate / ContextFillUpdate produce no scrollback line
  #   3. ToolCall / ToolResult / ToolProgress chunks produce no scrollback line (deferred to a future richer renderer)
  #   4. Suppressed chunks do not bump the scrollback_next_seq cursor
  #   5. chunk_to_lines is an exhaustive match with no catch-all — adding a new StreamChunk variant must fail to compile until classified
  #   6. IncomingMessage chunks render as `supervisor> {text}`
  #   7. Interrupted chunks render as `[interrupted] {n} queued`
  #
  # EXAMPLES:
  #   1. Screenshot reproduction: UserInput('please review this card'), SessionStateChange(Running), SessionStateChange(Idle) -> scrollback contains exactly one line `user> please review this card`
  #   2. UserInput('hello') -> scrollback line equals `user> hello`, seq advances by 1
  #   3. Two SessionStateChange chunks in a row leave scrollback empty and seq cursor at 0
  #   4. Interleaved: UserInput, SessionStateChange(Running), Text('hello back'), SessionStateChange(Idle), Done -> 3 scrollback lines (`user> ...`, `assistant> hello back`, `[done]`), seq=3
  #   5. IncomingMessage('supervisor here') -> scrollback line equals `supervisor> supervisor here`
  #   6. Interrupted{queued_inputs: vec!["a".into(), "b".into()]} -> scrollback line equals `[interrupted] 2 queued`
  #   7. All 14 state-only/tool variants fed in sequence -> scrollback empty, seq cursor at 0
  #   8. Status pill still reflects Idle after SessionStateChange chunks despite no scrollback line (state mutation still runs)
  #
  # ========================================
  Background: User Story
    As a fspec user with an AgentView open
    I want to submit a message and watch state changes flow
    So that I see only human-readable conversation lines and never raw Rust Debug output in my scrollback

  Scenario: Screenshot reproduction renders only the user input line
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards UserInput { text: "please review this card" } for s-1
    Then the s-1 scrollback contains exactly one line equal to "user> please review this card"
    When the chunks subscriber forwards SessionStateChange { state: Running } for s-1
    When the chunks subscriber forwards SessionStateChange { state: Idle } for s-1
    Then no scrollback line contains the literal substring "SessionStateChange" or "UserInput {"

  Scenario: UserInput chunk renders as user-prefix scrollback line
    Given a fresh SessionContext for session s-1
    When record_chunk receives StreamChunk::UserInput { text: "hello" }
    Then the scrollback contains exactly one rendered chunk
    Then the rendered text equals "user> hello"
    Then the scrollback_next_seq cursor equals 1

  Scenario: Two SessionStateChange chunks leave scrollback empty and seq at zero
    Given a fresh SessionContext for session s-1
    When record_chunk receives StreamChunk::SessionStateChange { state: Running }
    Then the scrollback is empty
    When record_chunk receives StreamChunk::SessionStateChange { state: Idle }
    Then the scrollback_next_seq cursor remains 0

  Scenario: Interleaved visible and silent chunks keep seq monotonic over visible chunks only
    Given a fresh SessionContext for session s-1
    When record_chunk receives UserInput { text: "hi" }, SessionStateChange(Running), Text { text: "hello back" }, SessionStateChange(Idle), Done in that order
    Then the scrollback contains exactly 3 rendered chunks
    Then the rendered scrollback lines equal ["user> hi", "assistant> hello back", "[done]"]
    Then the scrollback_next_seq cursor equals 3

  Scenario: IncomingMessage chunk renders as supervisor-prefix line
    Given a fresh SessionContext for session s-1
    When record_chunk receives StreamChunk::IncomingMessage { text: "supervisor here", images: None }
    Then the scrollback contains exactly one rendered chunk whose line equals "supervisor> supervisor here"

  Scenario: Interrupted chunk renders as interrupted-with-queued-count line
    Given a fresh SessionContext for session s-1
    When record_chunk receives StreamChunk::Interrupted { queued_inputs: vec!["a", "b"] }
    Then the scrollback contains exactly one rendered chunk whose line equals "[interrupted] 2 queued"

  Scenario: All state-only and tool variants are suppressed from scrollback
    Given a fresh SessionContext for session s-1
    When record_chunk receives one of each suppressed variant (SessionStateChange, IsolationStateChange, DebugStateChange, FooterStateUpdate, FspecCommandRequest, FspecCommandResult, WorkUnitsUpdate, SupervisorPendingInjection, CompactionComplete, TokenUpdate, ContextFillUpdate, ToolCall, ToolResult, ToolProgress)
    Then the scrollback is empty
    Then the scrollback_next_seq cursor remains 0

  Scenario: SessionStateChange still mutates per-session status pill even when suppressed from scrollback
    Given an App with an open session s-1
    When the App dispatches Action::ChunkReceived(s-1, SessionStateChange { state: Idle })
    Then the agent_view_store.session_status_for(&s-1) returns SessionStatus::Idle
    Then the s-1 scrollback remains empty
