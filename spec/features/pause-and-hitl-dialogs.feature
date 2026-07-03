@done
@multi-session
@pause-integration
@dialog
@rust
@tui
@agent-view
@rpc
@session-management
@persistence
@RPC-053
Feature: Pause / HITL UI (chunk-driven trigger + inline HITL slot end-to-end)
  """
  RPC-053 originally introduced a PauseDialog modal + HitlDialog modal. RPC-406
  superseded the pause side: the modal PauseDialog was DELETED and tool-approval
  pauses now render as an inline prompt in the input area (see
  spec/features/inline-tool-approval-pause-prompt.feature). RPC-411 superseded
  the HITL side the same way: the modal HitlDialog was DELETED and HITL requests
  now live in a per-session store slot rendered inline in the composer (see
  spec/features/inline-hitl-prompt.feature). This file retains the chunk-driven
  trigger contract, the Esc-cancel wire correctness, and error tolerance.
  Action variants in components/mod.rs: Action::HitlPromptFetched{session_id, request} (replaces the deleted Action::OpenHitlDialog — stores the fetched HitlRequest into the AgentViewStore per-session HITL slot), Action::PauseConfirmed{session_id, accept: bool}, Action::PauseTriple{session_id, choice: ApprovalChoice}, Action::PauseResumed{session_id} (kept for non-prompt callers; unreachable from the inline pause prompt), Action::HitlSubmitted{session_id, response: HitlResponse}, Action::HitlCancelled{session_id} (Esc outside Other mode — sends {cancelled:true, answers:[]} then clears the slot), Action::PauseCleared{session_id} (sent by chunk dispatcher on Running/Idle to clear the per-session pause AND HITL slots), Action::PauseStateFetched{session_id, state} (RPC-406 — stores the fetched PauseState into the AgentViewStore per-session slot)
  codelet/fspec-tui/src/app/dispatch_pause_hitl.rs hosts impl App methods: handle_pause_chunk(session_id) — fired from dispatch_stream_chunks SessionStateChange{Paused} arm, spawns parallel backend.get_pause_state + get_hitl_request and dispatches PauseStateFetched OR HitlPromptFetched (HITL wins on tie); handle_pause_confirmed/handle_pause_triple/handle_pause_resumed — fire-and-forget backend writes (the first two also clear the per-session pause slot); handle_hitl_submitted — fire-and-forget backend.send_hitl_response, clears the HITL slot (submit AND cancel both route here); handle_pause_cleared — clear the pause slot and the HITL slot for the session. RPC-411 reducers live in app/dispatch_hitl_prompt.rs.
  Wire into dispatch_stream_chunks::handle_stream_chunk_state_updates: branch SessionStateChange{state} on Running/Idle to dispatch Action::PauseCleared, AND on Paused to dispatch Action::PauseChunkReceived(session_id) which routes through handle_pause_chunk (the parallel get_pause_state/get_hitl_request fetcher). Route both actions via the try_dispatch_pause_hitl catch-all helper.
  MockBackend in tests/common/mod.rs carries: pause_state scripted Mutex<Option<PauseState>>, hitl_request scripted Mutex<Option<HitlRequest>>, per-call counters + last-call captures for pause_resume, pause_confirm, pause_triple, send_hitl_response, get_pause_state, get_hitl_request, plus error-injection helpers.
  Integration test file: codelet/fspec-tui/tests/pause_hitl_rpc053.rs with @step comments mapped 1:1 to feature scenarios.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # NOTE: The PauseDialog-modal rules/examples originally captured here were
  # superseded by RPC-406 (inline tool-approval pause prompt — see
  # spec/features/inline-tool-approval-pause-prompt.feature). The HitlDialog
  # modal rules/examples were superseded by RPC-411 (inline HITL prompt — see
  # spec/features/inline-hitl-prompt.feature). Remaining rules cover the
  # chunk-driven trigger, the Esc-cancel wire contract, and error tolerance.
  #
  # BUSINESS RULES:
  #   1. When the chunk dispatcher receives StreamChunk::SessionStateChange { state: Paused }, the App spawns backend.get_pause_state(session) AND backend.get_hitl_request(session) in parallel and dispatches Action::PauseStateFetched (per-session pause slot) when get_pause_state returns Some, Action::HitlPromptFetched (per-session HITL slot) when get_hitl_request returns Some, or nothing when both return None
  #   2. Action::HitlPromptFetched{session_id, request} stores the request into the AgentViewStore per-session HITL slot — no modal layer is ever mounted
  #   3. Esc on the inline HITL prompt (outside Other mode) sends HitlResponse{cancelled:true, answers:[]} via backend.send_hitl_response and clears the slot — the backend can never be stranded Paused by a dismissal
  #   4. Errors from backend.get_pause_state, backend.get_hitl_request, backend.pause_confirm, backend.pause_triple, backend.pause_resume, and backend.send_hitl_response are silently logged via tracing — no scrollback notice and no panic
  #   5. The inline pause prompt and the inline HITL prompt are mutually exclusive — when SessionStateChange{Paused} arrives and BOTH backend.get_pause_state and backend.get_hitl_request return Some, the HITL slot wins and no pause slot is set (the HITL handler in the agent loop sets hitl_request AND set_status(Paused); pause_state is only set by the tool-pause path)
  #   6. When SessionStateChange{Running} or SessionStateChange{Idle} arrives, the per-session pause slot AND HITL slot are cleared
  #
  # EXAMPLES:
  #   1. Agent loop's request_user_input handler sets hitl_request and emits SessionStateChange{Paused}; the dispatcher polls and stores the HITL slot with 2 options 'Yes' and 'No'; no compositor layer is mounted
  #   2. The inline HITL prompt is showing; user presses Esc → backend.send_hitl_response(session, HitlResponse{cancelled:true, answers:[]}) is called exactly once and the slot clears
  #   3. SessionStateChange{Paused} arrives but BOTH backend.get_pause_state and backend.get_hitl_request return None (race/stale chunk) → no slot is set and no further backend call is made
  #   4. Backend.get_hitl_request returns Err('decode failed') after SessionStateChange{Paused} arrives → no HITL slot is set; the App must not panic and no scrollback notice fires
  #
  # ========================================
  Background: User Story
    As a user of the Rust ratatui AgentView
    I want pause (tool-approval) and HITL (request_user_input) prompts to round-trip through the FspecBackend trait from chunk-driven per-session store slots
    So that I can approve or deny tool calls and answer agent questions without leaving the TUI or stalling the agent loop

  # ─────────────────────────────────────────────────────────────────────
  # Chunk-driven trigger: SessionStateChange { Paused } sets the right slot
  # ─────────────────────────────────────────────────────────────────────
  Scenario: SessionStateChange{Paused} with hitl_request Some stores a HITL slot
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return None for s-1
    And the MockBackend's get_hitl_request is scripted to return Some HitlRequest{ id: "q-1", question: "Continue?", header: "Apply changes?", options: [Yes, No] } for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then the AgentViewStore HITL slot for s-1 holds the fetched request
    And no modal layer is mounted on the Compositor
    And the AgentViewStore pause slot for s-1 is empty

  Scenario: SessionStateChange{Paused} with both pause_state Some and hitl_request Some — HITL wins
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return Some PauseState{ kind: Confirm, prompt: "p", tool_call_id: None } for s-1
    And the MockBackend's get_hitl_request is scripted to return Some HitlRequest{ id: "q-1", question: "Q", header: "H", options: [Yes, No] } for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then the AgentViewStore HITL slot for s-1 holds the fetched request
    And the AgentViewStore pause slot for s-1 is empty

  Scenario: SessionStateChange{Paused} with both returning None sets no slot
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return None for s-1
    And the MockBackend's get_hitl_request is scripted to return None for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then the AgentViewStore pause slot for s-1 is empty
    And the AgentViewStore HITL slot for s-1 is empty

  Scenario: SessionStateChange{Idle} clears the HITL slot
    Given an App with a MockBackend
    And session s-1 is the current session
    And session s-1 has an active HITL slot
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Idle }) is dispatched
    Then the AgentViewStore HITL slot for s-1 is empty
    And backend.send_hitl_response is NEVER called

  # ─────────────────────────────────────────────────────────────────────
  # Esc-cancel wire correctness (the stranding-bug fix)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Esc on the inline HITL prompt cancels through the backend
    Given an App with a MockBackend
    And session s-1 is the current session
    And session s-1 has an active HITL slot
    When the user presses Esc on the inline HITL prompt
    And all pending tasks have drained
    Then backend.send_hitl_response is called exactly once with (s-1, HitlResponse{ cancelled: true, answers: [] })
    And the AgentViewStore HITL slot for s-1 is empty

  # ─────────────────────────────────────────────────────────────────────
  # Error tolerance
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Backend.get_hitl_request error during the chunk-driven probe sets no slot
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return None for s-1
    And the MockBackend's get_hitl_request is scripted to return Err("decode failed") for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then the App must not panic
    And the AgentViewStore HITL slot for s-1 is empty
    And the AgentViewStore pause slot for s-1 is empty
    And the session s-1 scrollback contains no chunks mentioning "get_hitl_request"

  Scenario: Backend.send_hitl_response error on Esc-cancel still clears the slot and emits no scrollback notice
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's send_hitl_response is scripted to return Err("decode failed")
    And session s-1 has an active HITL slot
    When the user presses Esc on the inline HITL prompt
    And all pending tasks have drained
    Then the App must not panic
    And the AgentViewStore HITL slot for s-1 is empty
    And the session s-1 scrollback contains no chunks mentioning "send_hitl_response"

  # ─────────────────────────────────────────────────────────────────────
  # Source shape
  # ─────────────────────────────────────────────────────────────────────
  Scenario: codelet/fspec-tui/src/app/dispatch_pause_hitl.rs hosts the new pause/HITL helpers
    Given the file codelet/fspec-tui/src/app/dispatch_pause_hitl.rs exists
    When the file is compiled as part of codelet-fspec-tui
    Then it must declare impl App methods named handle_pause_chunk, handle_pause_confirmed, handle_pause_triple, handle_pause_resumed, handle_hitl_submitted, and handle_pause_cleared
    And codelet/fspec-tui/src/app/mod.rs must declare pub mod dispatch_pause_hitl and pub mod dispatch_hitl_prompt
    And codelet/fspec-tui/src/components/mod.rs must declare no pub mod hitl_dialog and no pub mod pause_dialog
    And codelet/fspec-tui/src/app/dispatch.rs must NOT exceed 300 logical lines
