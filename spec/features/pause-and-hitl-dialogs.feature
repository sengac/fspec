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
Feature: Pause / HITL UI (ConfirmDialog + HitlDialog end-to-end)
  """
  RPC-053 originally introduced a PauseDialog modal + HitlDialog modal. RPC-406
  superseded the pause side: the modal PauseDialog was DELETED and tool-approval
  pauses now render as an inline prompt in the input area (see
  spec/features/inline-tool-approval-pause-prompt.feature). This file retains
  the HITL modal behavior and the chunk-driven trigger contract.
  Action variants in components/mod.rs: Action::OpenHitlDialog{session_id, request}, Action::PauseConfirmed{session_id, accept: bool}, Action::PauseTriple{session_id, choice: ApprovalChoice}, Action::PauseResumed{session_id} (kept for non-prompt callers; unreachable from the inline pause prompt), Action::HitlSubmitted{session_id, response: HitlResponse}, Action::PauseCleared{session_id} (sent by chunk dispatcher on Running/Idle to pop any mounted HitlDialog and clear the per-session pause slot), Action::PauseStateFetched{session_id, state} (RPC-406 — stores the fetched PauseState into the AgentViewStore per-session slot)
  codelet/fspec-tui/src/components/hitl_dialog.rs hosts HitlDialog (priority Critical, uses Accent::Cyan via dialog_theme::render_dialog). Renders question as title, header as first body row, options as labelled rows (hotkey letter prefix), and an optional free-text input row when allow_text_input. Tab/Down advances selection (looping through options + free-text row when present), Enter submits, hotkey letters (a..z, case-insensitive) jump directly to the matching option AND submit, Esc dismisses without submit. HITL_DIALOG_ID constant for idempotent push and self-pop.
  codelet/fspec-tui/src/app/dispatch_pause_hitl.rs hosts impl App methods: handle_pause_chunk(session_id) — fired from dispatch_stream_chunks SessionStateChange{Paused} arm, spawns parallel backend.get_pause_state + get_hitl_request and dispatches PauseStateFetched OR OpenHitlDialog (HITL wins on tie); handle_open_hitl_dialog(session, request) — idempotent compositor push; handle_pause_confirmed/handle_pause_triple/handle_pause_resumed — fire-and-forget backend writes (the first two also clear the per-session pause slot); handle_hitl_submitted — fire-and-forget backend.send_hitl_response, pop dialog before await; handle_pause_cleared — clear the pause slot and pop any mounted HitlDialog for the session.
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
  # spec/features/inline-tool-approval-pause-prompt.feature). Remaining
  # rules cover the chunk-driven trigger + HITL modal only.
  #
  # BUSINESS RULES:
  #   1. When the chunk dispatcher receives StreamChunk::SessionStateChange { state: Paused }, the App spawns backend.get_pause_state(session) AND backend.get_hitl_request(session) in parallel and dispatches Action::PauseStateFetched (per-session store slot) when get_pause_state returns Some, Action::OpenHitlDialog when get_hitl_request returns Some, or nothing when both return None
  #   2. Action::OpenHitlDialog{session_id, request} pushes a fresh HitlDialog at Priority::Critical onto the Compositor, idempotent on dialog-id collision
  #   3. HitlDialog renders the request.question as title, request.header as body text, and one row per HitlOption labelled with a hotkey letter (a, b, c, ...) plus the option label and description
  #   4. When request.allow_text_input is true HitlDialog renders an additional free-text input row below the options that gains focus on Tab cycling and accepts text edit keystrokes
  #   5. Pressing the hotkey letter for an option directly submits HitlResponse{ id: request.id, value: option.label } via backend.send_hitl_response and pops the dialog; Enter on the highlighted option does the same
  #   6. Enter while the free-text row is focused submits HitlResponse{ id: request.id, value: <typed text> } via backend.send_hitl_response and pops the dialog
  #   7. Esc on HitlDialog pops the dialog but does NOT submit a response (the agent loop stays blocked on wait_for_hitl_response until the user submits OR a new HITL request supersedes)
  #   8. Errors from backend.get_pause_state, backend.get_hitl_request, backend.pause_confirm, backend.pause_triple, backend.pause_resume, and backend.send_hitl_response are silently logged via tracing — no scrollback notice and no panic
  #   9. The inline pause prompt and HitlDialog are mutually exclusive — when SessionStateChange{Paused} arrives and BOTH backend.get_pause_state and backend.get_hitl_request return Some, the HitlDialog wins and no pause slot is set (the HITL handler in the agent loop sets hitl_request AND set_status(Paused); pause_state is only set by the tool-pause path)
  #   10. When SessionStateChange{Running} or SessionStateChange{Idle} arrives, any mounted HitlDialog for that session is popped from the Compositor and the per-session pause slot is cleared
  #
  # EXAMPLES:
  #   1. Agent loop's request_user_input handler sets hitl_request and emits SessionStateChange{Paused}; the dispatcher polls and opens HitlDialog with 2 options 'Yes' and 'No'; user presses 'a' (hotkey for 'Yes') → backend.send_hitl_response(session, HitlResponse{id:req.id, value:'Yes'}) is called exactly once and the dialog is removed
  #   2. HitlDialog with allow_text_input=true is open; user presses Tab until the free-text row is focused, types 'maybe later', and presses Enter → backend.send_hitl_response(session, HitlResponse{id:req.id, value:'maybe later'}) is called exactly once and the dialog is removed
  #   3. HitlDialog is open; user presses Esc → the dialog is removed; backend.send_hitl_response is NEVER called (the agent loop stays blocked until either a new HITL request arrives OR a follow-up user submit fires)
  #   4. SessionStateChange{Paused} arrives but BOTH backend.get_pause_state and backend.get_hitl_request return None (race/stale chunk) → no dialog is pushed, no pause slot is set, and no further backend call is made
  #   5. Backend.get_hitl_request returns Err('decode failed') after SessionStateChange{Paused} arrives → no HitlDialog is mounted; the App must not panic and no scrollback notice fires
  #
  # ========================================
  Background: User Story
    As a user of the Rust ratatui AgentView
    I want to see and answer pause (tool-approval) and HITL (request_user_input) prompts via dedicated dialogs that round-trip through the FspecBackend trait
    So that I can approve or deny tool calls and answer agent questions without leaving the TUI or stalling the agent loop

  # ─────────────────────────────────────────────────────────────────────
  # Chunk-driven trigger: SessionStateChange { Paused } opens the right dialog
  # ─────────────────────────────────────────────────────────────────────
  Scenario: SessionStateChange{Paused} with hitl_request Some opens a HitlDialog
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return None for s-1
    And the MockBackend's get_hitl_request is scripted to return Some HitlRequest{ id: "q-1", question: "Continue?", header: "Apply changes?", options: [Yes, No], allow_text_input: false } for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then the Compositor contains a layer with id HITL_DIALOG_ID
    And the AgentViewStore pause slot for s-1 is empty

  Scenario: SessionStateChange{Paused} with both pause_state Some and hitl_request Some — HITL wins
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return Some PauseState{ kind: Confirm, prompt: "p", tool_call_id: None } for s-1
    And the MockBackend's get_hitl_request is scripted to return Some HitlRequest{ id: "q-1", question: "Q", header: "H", options: [Yes, No], allow_text_input: false } for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then the Compositor contains a layer with id HITL_DIALOG_ID
    And the AgentViewStore pause slot for s-1 is empty

  Scenario: SessionStateChange{Paused} with both returning None pushes no dialog
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return None for s-1
    And the MockBackend's get_hitl_request is scripted to return None for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then the AgentViewStore pause slot for s-1 is empty
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID

  Scenario: SessionStateChange{Idle} pops any mounted HitlDialog
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Idle }) is dispatched
    Then the Compositor does NOT contain a layer with id HITL_DIALOG_ID

  # ─────────────────────────────────────────────────────────────────────
  # HitlDialog: option selection (no free-text)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: HitlDialog renders one row per option with hotkey letters a, b, c
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with options ["Yes", "No", "Maybe"] and allow_text_input=false
    Then the HitlDialog's rows include hotkey letters "a", "b", "c"
    And the HitlDialog's option labels include "Yes", "No", "Maybe"

  Scenario: Pressing the hotkey letter "a" submits the matching option
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with request id "q-1" and options ["Yes", "No"] and allow_text_input=false
    When the user presses the character "a" on the HitlDialog
    And all pending tasks have drained
    Then backend.send_hitl_response is called exactly once with (s-1, HitlResponse{ id: "q-1", value: "Yes" })
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID

  Scenario: Enter on the highlighted option submits that option
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with request id "q-1" and options ["Yes", "No"] and allow_text_input=false
    And the HitlDialog's selected row is index 1 (option "No")
    When the user presses Enter on the HitlDialog
    And all pending tasks have drained
    Then backend.send_hitl_response is called exactly once with (s-1, HitlResponse{ id: "q-1", value: "No" })
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID

  Scenario: Esc on a HitlDialog dismisses without submitting
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1
    When the user presses Esc on the HitlDialog
    And all pending tasks have drained
    Then backend.send_hitl_response is NEVER called
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID

  # ─────────────────────────────────────────────────────────────────────
  # HitlDialog: free-text input
  # ─────────────────────────────────────────────────────────────────────
  Scenario: HitlDialog with allow_text_input=true renders a free-text input row below the options
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with options ["Yes", "No"] and allow_text_input=true
    Then the HitlDialog contains a free-text input row
    And the free-text input row's value is empty

  Scenario: Tab cycles past the options into the free-text input row
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with options ["Yes", "No"] and allow_text_input=true
    And the HitlDialog's selected row is the first option (index 0)
    When the user presses Tab on the HitlDialog three times
    Then the HitlDialog's selected row is the free-text input row

  Scenario: Typing into the free-text row updates its value
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with allow_text_input=true
    And the HitlDialog's selected row is the free-text input row
    When the user types "maybe later" into the free-text row
    Then the free-text row's value equals "maybe later"

  Scenario: Enter on the free-text row submits HitlResponse with the typed text
    Given an App with a MockBackend
    And session s-1 is the current session
    And a HitlDialog is mounted on the Compositor for s-1 with request id "q-1" and allow_text_input=true
    And the HitlDialog's free-text row contains "maybe later"
    And the HitlDialog's selected row is the free-text input row
    When the user presses Enter on the HitlDialog
    And all pending tasks have drained
    Then backend.send_hitl_response is called exactly once with (s-1, HitlResponse{ id: "q-1", value: "maybe later" })
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID

  # ─────────────────────────────────────────────────────────────────────
  # Error tolerance
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Backend.get_hitl_request error during the chunk-driven probe pushes no dialog
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's get_pause_state is scripted to return None for s-1
    And the MockBackend's get_hitl_request is scripted to return Err("decode failed") for s-1
    When Action::ChunkReceived(s-1, SessionStateChange{ state: Paused }) is dispatched
    And all pending tasks have drained
    Then the App must not panic
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    And the AgentViewStore pause slot for s-1 is empty
    And the session s-1 scrollback contains no chunks mentioning "get_hitl_request"

  Scenario: Backend.send_hitl_response error after Enter still pops the dialog and emits no scrollback notice
    Given an App with a MockBackend
    And session s-1 is the current session
    And the MockBackend's send_hitl_response is scripted to return Err("decode failed")
    And a HitlDialog is mounted on the Compositor for s-1 with request id "q-1" and options ["Yes"]
    When the user presses the character "a" on the HitlDialog
    And all pending tasks have drained
    Then the App must not panic
    And the Compositor does NOT contain a layer with id HITL_DIALOG_ID
    And the session s-1 scrollback contains no chunks mentioning "send_hitl_response"

  # ─────────────────────────────────────────────────────────────────────
  # Source shape
  # ─────────────────────────────────────────────────────────────────────
  Scenario: codelet/fspec-tui/src/app/dispatch_pause_hitl.rs hosts the new pause/HITL helpers
    Given the file codelet/fspec-tui/src/app/dispatch_pause_hitl.rs exists
    When the file is compiled as part of codelet-fspec-tui
    Then it must declare impl App methods named handle_pause_chunk, handle_open_hitl_dialog, handle_pause_confirmed, handle_pause_triple, handle_pause_resumed, handle_hitl_submitted, and handle_pause_cleared
    And codelet/fspec-tui/src/app/mod.rs must declare pub mod dispatch_pause_hitl
    And codelet/fspec-tui/src/components/mod.rs must declare pub mod hitl_dialog and no pub mod pause_dialog
    And codelet/fspec-tui/src/app/dispatch.rs must NOT exceed 300 logical lines
